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
        let privileges = match user.role {
            Role::App => "SELECT, INSERT, UPDATE, DELETE",
            Role::Migration => "CREATE, DROP, INDEX, ALTER, EXECUTE, SELECT, INSERT, UPDATE, DELETE",
            Role::Viewer => "SELECT",
            Role::Replication => "REPLICATION SLAVE, SELECT, RELOAD, REPLICATION CLIENT, LOCK TABLES, EXECUTE",
        };

        info!(user = user.name, privileges, "grant user privileges");

        match user.role {
            Role::Migration | Role::Replication => {
                // for REPLICATION, scope is global, otherwise "ERROR 1221 (HY000): Incorrect usage of DB GRANT and GLOBAL PRIVILEGES"
                let statement = format!("GRANT {} ON *.* TO '{}'@'%'", privileges, user.name);
                sqlx::query(&statement).execute(&self.pool).await?;
            }
            _ => {
                let target_dbs = if let Some(db) = &user.db { &[db.to_owned()] } else { dbs };
                for db in target_dbs {
                    let statement = format!("GRANT {privileges} ON `{db}`.* TO '{}'@'%'", user.name);
                    sqlx::query(&statement).execute(&self.pool).await?;
                }
            }
        }
        Ok(())
    }
}
