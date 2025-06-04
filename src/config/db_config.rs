use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct DBConfig {
    pub project: String,
    pub env: String,
    pub instance: String,
    #[serde(rename(deserialize = "type"))]
    pub db_type: DBType,
    #[serde(rename(deserialize = "rootSecret"))]
    pub root_secret: String,
    pub dbs: Vec<String>,
    pub users: Vec<User>,
    pub endpoint: Endpoint,
}

#[derive(Deserialize, Debug)]
pub enum DBType {
    MySQL,
    PostgreSQL,
}

impl DBConfig {
    pub fn validate(&self) {
        for user in &self.users {
            if matches!(self.db_type, DBType::MySQL) && user.name.len() > 32 {
                panic!("db user name must be no longer than 32, user={}", user.name);
            }
            if let (Auth::Password, None) = (&user.auth, &user.secret) {
                panic!("db password user must have secret, user={}", user.name);
            }
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
