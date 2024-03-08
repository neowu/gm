use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::Deserialize;

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

#[derive(Deserialize, Debug)]
struct Kube {
    name: String,
    zone: String,
}

#[derive(Deserialize, Debug)]
struct User {
    name: String,
    auth: Auth,
    secret: Option<String>,
    db: Option<String>,
    role: Role,
}

#[derive(Deserialize, Debug)]
enum Auth {
    IAM,
    PASSWORD,
}

#[derive(Deserialize, Debug)]
enum Role {
    APP,
    MIGRATION,
    VIEWER,
    REPLICATION,
}

impl DBConfig {
    pub(crate) fn validate(&self) -> Result<(), Box<dyn Error>> {
        for user in &self.users {
            if user.name.len() > 32 {
                return Err(Box::new(ConfigError {
                    message: format!("db user name must be no longer than 32, user={}", user.name),
                }));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ConfigError {
    message: String,
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ConfigError {
    fn description(&self) -> &str {
        &self.message
    }
}
