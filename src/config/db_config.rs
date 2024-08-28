use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct DBConfig {
    pub project: String,
    pub env: String,
    pub instance: String,
    #[serde(rename(deserialize = "rootSecret"))]
    pub root_secret: String,
    pub dbs: Vec<String>,
    pub users: Vec<User>,
    pub endpoints: Vec<Endpoint>,
}

impl DBConfig {
    pub fn validate(&self) {
        for user in &self.users {
            if user.name.len() > 32 {
                panic!("db user name must be no longer than 32, user={}", user.name);
            }
            if let (Auth::Password, None) = (&user.auth, &user.secret) {
                panic!("db password user must have secret, user={}", user.name);
            }
        }
    }

    pub fn dbs<'a>(&'a self, user: &'a User) -> Vec<&'a str> {
        if matches!(user.role, Role::Migration) || matches!(user.role, Role::Replication) {
            return vec!["*"]; // for REPLICATION, scope is global, otherwise "ERROR 1221 (HY000): Incorrect usage of DB GRANT and GLOBAL PRIVILEGES"
        }

        match &user.db {
            Some(db) => vec![db],
            None => self.dbs.iter().map(|s| s.as_str()).collect(),
        }
    }
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
            Role::App => vec!["SELECT", "INSERT", "UPDATE", "DELETE"],
            Role::Migration => vec!["CREATE", "DROP", "INDEX", "ALTER", "EXECUTE", "SELECT", "INSERT", "UPDATE", "DELETE"],
            Role::Viewer => vec!["SELECT"],
            Role::Replication => vec!["REPLICATION SLAVE", "SELECT", "RELOAD", "REPLICATION CLIENT", "LOCK TABLES", "EXECUTE"],
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Endpoint {
    pub name: String,
    pub ns: String,
    pub path: String,
}

#[derive(Deserialize, Debug)]
pub enum Auth {
    #[serde(rename(deserialize = "IAM"))]
    Iam,
    #[serde(rename(deserialize = "PASSWORD"))]
    Password,
}

#[derive(Deserialize, Debug)]
pub enum Role {
    #[serde(rename(deserialize = "APP"))]
    App,
    #[serde(rename(deserialize = "MIGRATION"))]
    Migration,
    #[serde(rename(deserialize = "VIEWER"))]
    Viewer,
    #[serde(rename(deserialize = "REPLICATION"))]
    Replication,
}
