use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(super) struct DBConfig {
    project: String,
    env: String,
    instance: String,
    kube: Kube,
    #[serde(rename(deserialize = "rootSecret"))]
    root_secret: String,
    dbs: Vec<String>,
    users: Vec<User>,
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
