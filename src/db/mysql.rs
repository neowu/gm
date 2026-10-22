use std::slice::from_ref;

use anyhow::Result;
use sqlx::MySql;
use sqlx::Pool;
use sqlx::mysql::MySqlConnectOptions;
use tracing::info;

use crate::config::db_config::Role;
use crate::config::db_config::User;

pub struct MySQL {
    pool: Pool<MySql>,
}

impl MySQL {
    pub async fn new(public_ip: &str, user: &str, password: &str) -> Result<Self> {
        let options = format!("mysql://{public_ip}")
            .parse::<MySqlConnectOptions>()?
            .username(user)
            .password(password);
        let pool = Pool::<MySql>::connect_with(options).await?;
        Ok(MySQL { pool })
    }

    pub async fn create_db(&mut self, db: &str) -> Result<()> {
        info!(db, "create db");
        let statement = format!("CREATE DATABASE IF NOT EXISTS `{db}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci");
        sqlx::query(&statement).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn create_user(&mut self, user: &str, password: &str) -> Result<()> {
        info!(user, "create user");
        let statement = format!("CREATE USER IF NOT EXISTS '{user}'@'%'");
        sqlx::query(&statement).execute(&self.pool).await?;

        let statement = format!("ALTER USER '{user}'@'%' IDENTIFIED BY '{password}'");
        sqlx::query(&statement).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn grant_user_privileges(&mut self, user: &User, dbs: &[String]) -> Result<()> {
        let privileges = privileges(&user.role);
        info!(user = user.name, privileges, "grant user privileges");
        for scope in scopes(user, dbs) {
            let statement = format!("GRANT {privileges} ON {scope} TO '{}'@'%'", user.name);
            sqlx::query(&statement).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn revoke_user_privileges(&mut self, user: &str) -> Result<()> {
        info!(user, "revoke user privileges");
        let statement = format!("REVOKE ALL PRIVILEGES, GRANT OPTION FROM '{user}'@'%'");
        sqlx::query(&statement).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn drop_user(&mut self, user: &str) -> Result<()> {
        info!(user, "drop user");
        let statement = format!("DROP USER '{user}'@'%'");
        sqlx::query(&statement).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn db_exists(&mut self, db: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM information_schema.schemata WHERE schema_name = ?")
            .bind(db)
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    pub async fn current_grants(&mut self, user: &User, dbs: &[String]) -> Result<Option<Vec<String>>> {
        let target = target_grants(user, dbs);
        let user = &user.name;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM mysql.user WHERE user = ? AND host = '%'")
            .bind(user)
            .fetch_one(&self.pool)
            .await?;
        if count == 0 {
            return Ok(None);
        }
        let statements: Vec<String> = sqlx::query_scalar(&format!("SHOW GRANTS FOR '{user}'@'%'")).fetch_all(&self.pool).await?;
        let current: Vec<String> = statements.iter().flat_map(|s| grant_elements(s)).collect();
        Ok(Some(
            current
                .iter()
                .filter(|grant| target.contains(grant) || !implied(grant, &current))
                .cloned()
                .collect(),
        ))
    }
}

pub fn target_grants(user: &User, dbs: &[String]) -> Vec<String> {
    let privileges = privileges(&user.role);
    scopes(user, dbs)
        .into_iter()
        .flat_map(|scope| privileges.split(", ").map(move |privilege| format!("{privilege} ON {scope}")))
        .collect()
}

fn privileges(role: &Role) -> &'static str {
    match role {
        Role::App => "SELECT, INSERT, UPDATE, DELETE",
        Role::Migration => "CREATE, DROP, INDEX, ALTER, EXECUTE, SELECT, INSERT, UPDATE, DELETE",
        Role::Viewer => "SELECT",
        Role::Replication => "REPLICATION SLAVE, SELECT, RELOAD, REPLICATION CLIENT, LOCK TABLES, EXECUTE",
        Role::Super => panic!("MySQL doesn't support Super role"),
    }
}

// same format as SHOW GRANTS, e.g. *.* or `db`.*
fn scopes(user: &User, dbs: &[String]) -> Vec<String> {
    match user.role {
        // for REPLICATION, scope is global, otherwise "ERROR 1221 (HY000): Incorrect usage of DB GRANT and GLOBAL PRIVILEGES"
        Role::Migration | Role::Replication => vec!["*.*".to_owned()],
        _ => {
            let target_dbs = if let Some(db) = &user.db { from_ref(db) } else { dbs };
            target_dbs.iter().map(|db| format!("`{db}`.*")).collect()
        }
    }
}

// db scope privilege covered by same global privilege (e.g. left by old gm granting MIGRATION per db),
// or FLUSH_PRIVILEGES auto granted by mysql upgrade to users with global RELOAD
fn implied(grant: &str, current: &[String]) -> bool {
    let global = |privilege: &str| current.iter().any(|g| g == &format!("{privilege} ON *.*"));
    match grant.split_once(" ON ") {
        Some(("FLUSH_PRIVILEGES", "*.*")) => global("RELOAD"),
        Some((privilege, scope)) => scope != "*.*" && global(privilege),
        None => false,
    }
}

// split SHOW GRANTS statement into "{privilege} ON {scope}", USAGE is skipped as it means no privilege
// SHOW_ROUTINE is skipped, it's auto granted by mysql upgrade to users with global SELECT, and cloud sql root can't revoke it
// partial revokes on system schemas are added by cloud sql to users with global privileges, skipped
fn grant_elements(statement: &str) -> Vec<String> {
    if statement.starts_with("REVOKE ") && (statement.contains(" ON `mysql`.* ") || statement.contains(" ON `sys`.* ")) {
        return vec![];
    }
    let Some((body, to)) = statement.strip_prefix("GRANT ").and_then(|s| s.rsplit_once(" TO ")) else {
        return vec![statement.to_owned()];
    };
    let option = to.find(" WITH ").map(|i| &to[i..]).unwrap_or("");
    match body.rsplit_once(" ON ") {
        Some((privileges, scope)) => privileges
            .split(',')
            .map(str::trim)
            .filter(|privilege| *privilege != "USAGE" && *privilege != "SHOW_ROUTINE")
            .map(|privilege| format!("{privilege} ON {scope}{option}"))
            .collect(),
        None => vec![format!("{body}{option}")],
    }
}

#[cfg(test)]
mod tests {
    use super::grant_elements;
    use super::implied;

    #[test]
    fn split_show_grants() {
        assert!(grant_elements("GRANT USAGE ON *.* TO `app`@`%`").is_empty());
        assert!(grant_elements("REVOKE INSERT, UPDATE ON `mysql`.* FROM `app`@`%`").is_empty());
        assert!(grant_elements("GRANT SHOW_ROUTINE ON *.* TO `app`@`%`").is_empty());
        assert_eq!(
            grant_elements("GRANT SELECT, INSERT ON `db`.* TO `app`@`%`"),
            ["SELECT ON `db`.*", "INSERT ON `db`.*"]
        );
        assert_eq!(
            grant_elements("GRANT BACKUP_ADMIN,BINLOG_ADMIN ON *.* TO `app`@`%` WITH GRANT OPTION"),
            ["BACKUP_ADMIN ON *.* WITH GRANT OPTION", "BINLOG_ADMIN ON *.* WITH GRANT OPTION"]
        );
    }

    #[test]
    fn implied_by_global_grants() {
        let current = ["SELECT ON *.*".to_owned(), "RELOAD ON *.*".to_owned()];
        assert!(implied("SELECT ON `db`.*", &current));
        assert!(implied("FLUSH_PRIVILEGES ON *.*", &current));
        assert!(!implied("SELECT ON *.*", &current));
        assert!(!implied("INSERT ON `db`.*", &current));
    }
}
