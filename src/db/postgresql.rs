use std::slice::from_ref;

use anyhow::Result;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::config::db_config::Role;
use crate::config::db_config::User;

pub struct PostgreSQL {
    options: Box<PgConnectOptions>,
}

impl PostgreSQL {
    pub async fn new(public_ip: &str, user: &str, password: &str) -> Result<Self> {
        info!(user, "create postgres options");
        let options = PgConnectOptions::new().host(public_ip).username(user).password(password);
        Ok(PostgreSQL { options: Box::new(options) })
    }

    pub async fn create_db(&mut self, db: &str) -> Result<()> {
        let pool = self.pool("postgres").await?;

        info!(db, "check if db exists");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM pg_database WHERE datname = $1")
            .bind(db)
            .fetch_one(&pool)
            .await?;

        if count == 0 {
            info!(db, "create db");
            let statement = format!(r#"CREATE DATABASE "{db}""#);
            execute(&pool, statement).await?;
        }

        info!(db, "update db");
        let pool = self.pool(db).await?;

        let statements = [
            "CREATE EXTENSION IF NOT EXISTS pg_stat_statements".to_owned(),
            format!(r#"ALTER DATABASE "{db}" SET auto_explain.log_min_duration = 3000"#),
            format!(r#"ALTER DATABASE "{db}" SET auto_explain.log_analyze = true"#),
            format!(r#"ALTER DATABASE "{db}" SET auto_explain.log_buffers = true"#),
            format!(r#"ALTER DATABASE "{db}" SET auto_explain.log_nested_statements = true"#),
            format!(r#"ALTER DATABASE "{db}" SET auto_explain.log_settings = true"#),
            format!(r#"ALTER DATABASE "{db}" SET auto_explain.log_verbose = true"#),
            format!(r#"ALTER DATABASE "{db}" SET auto_explain.log_wal = true"#),
        ];
        execute_all(&pool, &statements).await?;

        Ok(())
    }

    pub async fn create_user(&mut self, user: &User, password: &str) -> Result<()> {
        let user_name = &user.name;
        info!(user = user_name, "check if user exists");

        let pool = self.pool("postgres").await?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM pg_user WHERE usename = $1")
            .bind(user_name)
            .fetch_one(&pool)
            .await?;

        if count == 0 {
            info!(user = user_name, "create user");

            if let Role::Replication = user.role {
                let statement = format!(r#"CREATE USER "{user_name}" WITH REPLICATION LOGIN PASSWORD '{password}'"#);
                execute(&pool, statement).await?;
            } else {
                let statement = format!(r#"CREATE USER "{user_name}" WITH PASSWORD '{password}'"#);
                execute(&pool, statement).await?;
            }
        }

        Ok(())
    }

    pub async fn grant_user_privileges(&mut self, user: &User, dbs: &[String]) -> Result<()> {
        info!(user = user.name, "grant user privileges");

        let user_name = &user.name;
        let privileges = privileges(&user.role);
        for db in target_dbs(user, dbs) {
            let pool = self.pool(db).await?;
            let mut statements = vec![format!(r#"GRANT {} ON DATABASE "{db}" TO "{user_name}""#, privileges.database.join(", "))];
            if !privileges.schema.is_empty() {
                statements.push(format!(r#"GRANT {} ON SCHEMA public TO "{user_name}""#, privileges.schema.join(", ")));
            }
            for role in privileges.roles {
                statements.push(format!(r#"GRANT {role} TO "{user_name}""#));
            }
            execute_all(&pool, &statements).await?;
        }
        Ok(())
    }

    pub async fn db_exists(&mut self, db: &str) -> Result<bool> {
        let pool = self.pool("postgres").await?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM pg_database WHERE datname = $1")
            .bind(db)
            .fetch_one(&pool)
            .await?;
        Ok(count > 0)
    }

    // only check privileges managed by sync on config dbs: database / public schema acl, role membership, replication attribute
    pub async fn current_grants(&mut self, user: &str, dbs: &[String]) -> Result<Option<Vec<String>>> {
        let pool = self.pool("postgres").await?;
        let Some(replication): Option<bool> = sqlx::query_scalar("SELECT rolreplication FROM pg_roles WHERE rolname = $1")
            .bind(user)
            .fetch_optional(&pool)
            .await?
        else {
            return Ok(None);
        };

        let mut grants = vec![];
        if replication {
            grants.push("REPLICATION".to_owned());
        }
        // cloudsqliamuser / cloudsqliamserviceaccount are assigned by gcloud to IAM users
        let roles: Vec<String> = sqlx::query_scalar(
            "SELECT r.rolname::text FROM pg_auth_members m JOIN pg_roles r ON m.roleid = r.oid JOIN pg_roles u ON m.member = u.oid
             WHERE u.rolname = $1 AND r.rolname NOT IN ('cloudsqliamuser', 'cloudsqliamserviceaccount') ORDER BY 1",
        )
        .bind(user)
        .fetch_all(&pool)
        .await?;
        grants.extend(roles.iter().map(|role| format!("ROLE {role}")));

        for db in dbs {
            if !self.db_exists(db).await? {
                continue;
            }
            let privileges: Vec<String> = sqlx::query_scalar(
                "SELECT a.privilege_type FROM pg_database d, aclexplode(d.datacl) a
                 WHERE d.datname = $1 AND a.grantee = (SELECT oid FROM pg_roles WHERE rolname = $2)",
            )
            .bind(db)
            .bind(user)
            .fetch_all(&pool)
            .await?;
            grants.extend(privileges.iter().map(|privilege| format!(r#"{privilege} ON DATABASE "{db}""#)));

            let privileges: Vec<String> = sqlx::query_scalar(
                "SELECT a.privilege_type FROM pg_namespace n, aclexplode(n.nspacl) a
                 WHERE n.nspname = 'public' AND a.grantee = (SELECT oid FROM pg_roles WHERE rolname = $1)",
            )
            .bind(user)
            .fetch_all(&self.pool(db).await?)
            .await?;
            grants.extend(privileges.iter().map(|privilege| format!(r#"{privilege} ON SCHEMA "{db}".public"#)));
        }
        Ok(Some(grants))
    }

    // revoke privileges managed by sync on config dbs, postgres refuses to drop role with database / schema acl
    pub async fn revoke_user_privileges(&mut self, user: &str, dbs: &[String]) -> Result<()> {
        info!(user, "revoke user privileges");
        let pool = self.pool("postgres").await?;
        let roles: Vec<String> = sqlx::query_scalar(
            "SELECT r.rolname::text FROM pg_auth_members m JOIN pg_roles r ON m.roleid = r.oid JOIN pg_roles u ON m.member = u.oid
             WHERE u.rolname = $1 AND r.rolname NOT IN ('cloudsqliamuser', 'cloudsqliamserviceaccount') ORDER BY 1",
        )
        .bind(user)
        .fetch_all(&pool)
        .await?;
        let statements: Vec<String> = roles.iter().map(|role| format!(r#"REVOKE "{role}" FROM "{user}""#)).collect();
        execute_all(&pool, &statements).await?;

        for db in dbs {
            if !self.db_exists(db).await? {
                continue;
            }
            let statements = [
                format!(r#"REVOKE ALL ON DATABASE "{db}" FROM "{user}""#),
                format!(r#"REVOKE ALL ON SCHEMA public FROM "{user}""#),
            ];
            execute_all(&self.pool(db).await?, &statements).await?;
        }
        Ok(())
    }

    pub async fn drop_user(&mut self, user: &str) -> Result<()> {
        let pool = self.pool("postgres").await?;
        execute(&pool, format!(r#"DROP ROLE "{user}""#)).await
    }

    async fn pool(&self, db: &str) -> Result<Pool<Postgres>> {
        let options = self.options.clone().database(db);
        Ok(PgPoolOptions::new().max_connections(1).connect_with(options).await?)
    }
}

pub fn target_grants(user: &User, dbs: &[String]) -> Vec<String> {
    let privileges = privileges(&user.role);
    let mut grants = vec![];
    if let Role::Replication = user.role {
        grants.push("REPLICATION".to_owned());
    }
    grants.extend(privileges.roles.iter().map(|role| format!("ROLE {role}")));
    for db in target_dbs(user, dbs) {
        grants.extend(privileges.database.iter().map(|privilege| format!(r#"{privilege} ON DATABASE "{db}""#)));
        grants.extend(
            privileges
                .schema
                .iter()
                .map(|privilege| format!(r#"{privilege} ON SCHEMA "{db}".public"#)),
        );
    }
    grants
}

struct Privileges {
    database: &'static [&'static str],
    schema: &'static [&'static str],
    roles: &'static [&'static str],
}

fn privileges(role: &Role) -> Privileges {
    match role {
        // migration user will be owners of all tables, thus has read/write access
        Role::Migration => Privileges {
            database: &["CREATE", "CONNECT"],
            schema: &["CREATE", "USAGE"],
            roles: &[],
        },
        Role::App => Privileges {
            database: &["CONNECT"],
            schema: &[],
            roles: &["pg_read_all_data", "pg_write_all_data"],
        },
        Role::Viewer => Privileges {
            database: &["CONNECT"],
            schema: &[],
            roles: &["pg_read_all_data"],
        },
        Role::Replication => Privileges {
            database: &["CONNECT"],
            schema: &["USAGE"],
            roles: &["pg_read_all_data"],
        },
        // refer to https://docs.cloud.google.com/sql/docs/postgres/users, cloudsqlsuperuser allows viewing query by `select * from pg_stat_activity`
        Role::Super => Privileges {
            database: &["CONNECT"],
            schema: &[],
            roles: &["pg_read_all_data", "pg_write_all_data", "cloudsqlsuperuser"],
        },
    }
}

fn target_dbs<'a>(user: &'a User, dbs: &'a [String]) -> &'a [String] {
    if let Some(db) = &user.db { from_ref(db) } else { dbs }
}

async fn execute(pool: &Pool<Postgres>, statement: String) -> Result<()> {
    info!(statement, "execute SQL");
    sqlx::query(&statement).execute(pool).await?;
    Ok(())
}

async fn execute_all(pool: &Pool<Postgres>, statements: &[String]) -> Result<()> {
    for statement in statements {
        info!(statement, "execute SQL");
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}
