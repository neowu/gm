use anyhow::Result;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::config::db_config::Role;
use crate::config::db_config::User;

pub struct PostgreSQL {
    pool: Pool<Postgres>,
}

impl PostgreSQL {
    pub async fn new(public_ip: &str, user: &str, password: &str) -> Result<Self> {
        info!(user, "connect to postgres");
        let options = format!("postgres://{public_ip}/postgres")
            .parse::<PgConnectOptions>()?
            .username(user)
            .password(password);
        let pool = PgPoolOptions::new().max_connections(5).connect_with(options).await?;
        Ok(PostgreSQL { pool })
    }

    pub async fn create_db(&mut self, db: &str) -> Result<()> {
        info!(db, "check if db exists");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM pg_database WHERE datname = $1")
            .bind(db)
            .fetch_one(&self.pool)
            .await?;

        if count == 0 {
            info!(db, "create db");
            sqlx::query(&format!(r#"CREATE DATABASE "{db}""#)).execute(&self.pool).await?;
        }

        Ok(())
    }

    pub async fn create_user(&mut self, user: &str, password: &str) -> Result<()> {
        info!(user, "check if user exists");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM pg_user WHERE usename = $1")
            .bind(user)
            .fetch_one(&self.pool)
            .await?;

        if count == 0 {
            info!(user, "create user");
            sqlx::query(&format!(r#"CREATE USER "{user}" WITH PASSWORD '{password}'"#))
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    pub async fn grant_user_privileges(&mut self, user: &User, dbs: &[String]) -> Result<()> {
        let privileges = match user.role {
            Role::App => "SELECT, INSERT, UPDATE, DELETE",
            // postgres does not support DROP, only owner of object can drop it
            Role::Migration => "CREATE, INDEX, ALTER, EXECUTE, SELECT, INSERT, UPDATE, DELETE",
            Role::Viewer => "SELECT",
            Role::Replication => todo!("not supported yet"),
        };

        info!(user = user.name, privileges, "grant user privileges");

        let target_dbs = if let Some(db) = &user.db { &[db.clone()] } else { dbs };
        for db in target_dbs {
            sqlx::query(&format!(r#"GRANT {privileges} ON DATABASE "{db}" TO "{}""#, user.name))
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}
