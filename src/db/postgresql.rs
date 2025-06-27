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

        let target_dbs = if let Some(db) = &user.db { &[db.to_owned()] } else { dbs };
        let user_name = &user.name;
        for db in target_dbs {
            let pool = self.pool(db).await?;

            match user.role {
                Role::Migration => {
                    // migration user will be owners of all tables, thus has read/write access
                    let statements = [
                        format!(r#"GRANT CREATE, CONNECT ON DATABASE "{db}" TO "{user_name}""#),
                        format!(r#"GRANT CREATE, USAGE ON SCHEMA public TO "{user_name}""#),
                    ];
                    execute_all(&pool, &statements).await?;
                }
                Role::App => {
                    let statements = [
                        format!(r#"GRANT CONNECT ON DATABASE "{db}" TO "{user_name}""#),
                        format!(r#"GRANT pg_write_all_data TO "{user_name}""#),
                    ];
                    execute_all(&pool, &statements).await?;
                }
                Role::Viewer => {
                    let statements = [
                        format!(r#"GRANT CONNECT ON DATABASE "{db}" TO "{user_name}""#),
                        format!(r#"GRANT pg_read_all_data TO "{user_name}""#),
                    ];
                    execute_all(&pool, &statements).await?;
                }
                Role::Replication => {
                    let statements = [
                        format!(r#"GRANT CONNECT ON DATABASE "{db}" TO "{user_name}""#),
                        format!(r#"GRANT USAGE ON SCHEMA public TO "{user_name}""#),
                        format!(r#"GRANT pg_read_all_data TO "{user_name}""#),
                    ];
                    execute_all(&pool, &statements).await?;
                }
            };
        }
        Ok(())
    }

    async fn pool(&self, db: &str) -> Result<Pool<Postgres>> {
        let options = self.options.clone().database(db);
        Ok(PgPoolOptions::new().max_connections(1).connect_with(options).await?)
    }
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
