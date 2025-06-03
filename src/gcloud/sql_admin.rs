use crate::gcloud;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

#[derive(Deserialize, Debug)]
pub struct GetSQLInstanceResponse {
    #[serde(rename(deserialize = "kind"))]
    _kind: String,
    #[serde(rename(deserialize = "ipAddresses"))]
    addresses: Vec<IPAddress>,
}

#[derive(Deserialize, Debug)]
pub struct IPAddress {
    #[serde(rename(deserialize = "ipAddress"))]
    address: String,
    r#type: String,
}

#[derive(Serialize, Debug)]
struct User {
    name: String,
    host: String,
    password: String,
}

#[derive(Deserialize, Debug)]
struct Operation {
    #[serde(rename(deserialize = "kind"))]
    _kind: String,
}

impl GetSQLInstanceResponse {
    pub fn public_address(&self) -> &str {
        self.addresses
            .iter()
            .find(|ip| ip.r#type == "PRIMARY")
            .map(|ip| ip.address.as_str())
            .expect("public ip should not be null")
    }

    pub fn private_address(&self) -> &str {
        self.addresses
            .iter()
            .find(|ip| ip.r#type == "PRIVATE")
            .map(|ip| ip.address.as_str())
            .expect("private ip should not be null")
    }
}

pub async fn get_sql_instance(project: &str, instance: &str) -> GetSQLInstanceResponse {
    let url = format!("https://sqladmin.googleapis.com/v1/projects/{project}/instances/{instance}");
    gcloud::get(&url)
        .await
        .unwrap_or_else(|| panic!("instance not found, instance={instance}"))
}

pub async fn set_root_password(project: &str, instance: &str, password: &str) {
    info!(instance, "change sql instance root password");
    let url = format!("https://sqladmin.googleapis.com/v1/projects/{project}/instances/{instance}/users");
    let _: Operation = gcloud::post(
        &url,
        &User {
            name: "root".to_owned(),
            host: "%".to_owned(),
            password: password.to_string(),
        },
    )
    .await;
}
