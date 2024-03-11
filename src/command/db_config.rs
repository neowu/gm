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
    pub(crate) fn validate(&self) -> Result<(), Exception> {
        for user in &self.users {
            if user.name.len() > 32 {
                return Err(Exception::new(&format!(
                    "db user name must be no longer than 32, user={}",
                    user.name
                )));
            }
        }
        Ok(())
    }
}
