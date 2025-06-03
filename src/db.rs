use anyhow::Result;
use mysql::MySQL;
use postgresql::PostgreSQL;

use crate::config::db_config::DBType;
use crate::config::db_config::User;

mod mysql;
mod postgresql;

pub enum Database {
    MySQL(MySQL),
    PostgreSQL(PostgreSQL),
}

impl Database {
    pub async fn create_database(db_type: &DBType, public_ip: &str, password: &str) -> Result<Database> {
        match db_type {
            DBType::MySQL => Ok(Database::MySQL(MySQL::new(public_ip, "root", password).await?)),
            DBType::PostgreSQL => Ok(Database::PostgreSQL(PostgreSQL::new(public_ip, "postgres", password).await?)),
        }
    }

    pub async fn create_db(&mut self, db: &str) -> Result<()> {
        match self {
            Database::MySQL(mysql) => mysql.create_db(db).await,
            Database::PostgreSQL(postgresql) => postgresql.create_db(db).await,
        }
    }

    pub async fn create_user(&mut self, user: &str, password: &str) -> Result<()> {
        match self {
            Database::MySQL(mysql) => mysql.create_user(user, password).await,
            Database::PostgreSQL(postgresql) => postgresql.create_user(user, password).await,
        }
    }

    pub async fn grant_user_privileges(&mut self, user: &User, dbs: &[String]) -> Result<()> {
        match self {
            Database::MySQL(mysql) => mysql.grant_user_privileges(user, dbs).await,
            Database::PostgreSQL(postgresql) => postgresql.grant_user_privileges(user, dbs).await,
        }
    }
}
