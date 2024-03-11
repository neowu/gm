use serde::Deserialize;

use crate::util::exception::Exception;

#[derive(Deserialize, Debug)]
pub(super) struct DBConfig {
    pub project: String,
    pub env: String,
    pub instance: String,
    pub kube: Kube,
    #[serde(rename(deserialize = "rootSecret"))]
    pub root_secret: String,
    pub dbs: Vec<String>,
    pub users: Vec<User>,
}

impl DBConfig {
    pub fn dbs(&self, user: &User) -> Vec<String> {
        if let Role::MIGRATION = user.role {
            return vec!["*".to_string()]; // for REPLICATION, scope is global, otherwise "ERROR 1221 (HY000): Incorrect usage of DB GRANT and GLOBAL PRIVILEGES"
        }

        match &user.db {
            Some(db) => vec![db.to_string()],
            None => self.dbs.clone(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Kube {
    pub name: String,
    pub zone: String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub name: String,
    pub auth: Auth,
    pub secret: Option<String>,
    pub db: Option<String>,
    pub role: Role,
}

impl User {
    pub fn privileges(&self) -> Vec<&str> {
        match self.role {
            Role::APP => vec!["SELECT", "INSERT", "UPDATE", "DELETE"],
            Role::MIGRATION => vec!["CREATE", "DROP", "INDEX", "ALTER", "EXECUTE", "SELECT", "INSERT", "UPDATE", "DELETE"],
            Role::VIEWER => vec!["SELECT"],
            Role::REPLICATION => vec!["REPLICATION SLAVE", "SELECT", "RELOAD", "REPLICATION CLIENT", "LOCK TABLES", "EXECUTE"],
        }
    }
}

#[derive(Deserialize, Debug)]
pub enum Auth {
    IAM,
    PASSWORD,
}

#[derive(Deserialize, Debug)]
pub enum Role {
    APP,
    MIGRATION,
    VIEWER,
    REPLICATION,
}

impl DBConfig {
    pub(crate) fn validate(&self) -> Result<(), Exception> {
        for user in &self.users {
            if user.name.len() > 32 {
                return Err(Exception::new(&format!("db user name must be no longer than 32, user={}", user.name)));
            }
        }
        Ok(())
    }
}
