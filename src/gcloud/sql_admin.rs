use crate::gcloud;
use crate::util::exception::Exception;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use super::GCloudError;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct GetSQLInstanceResponse {
    kind: String,
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

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Operation {
    kind: String,
}

impl GetSQLInstanceResponse {
    pub fn public_address(&self) -> Result<&str, Exception> {
        self.addresses
            .iter()
            .find(|ip| ip.r#type == "PRIMARY")
            .map(|ip| ip.address.as_str())
            .ok_or(Exception::new("public ip must not be null"))
    }

    pub fn private_address(&self) -> Result<&str, Exception> {
        self.addresses
            .iter()
            .find(|ip| ip.r#type == "PRIVATE")
            .map(|ip| ip.address.as_str())
            .ok_or(Exception::new("private ip must not be null"))
    }
}

pub async fn get_sql_instance(project: &str, instance: &str) -> Result<GetSQLInstanceResponse, GCloudError> {
    let url = format!("https://sqladmin.googleapis.com/v1/projects/{project}/instances/{instance}");
    gcloud::get(&url).await
}

pub async fn set_root_password(project: &str, instance: &str, password: &str) -> Result<(), GCloudError> {
    info!("change sql instance root password, instance={instance}");
    let url = format!("https://sqladmin.googleapis.com/v1/projects/{project}/instances/{instance}/users");
    let _: Operation = gcloud::post(
        &url,
        &User {
            name: "root".to_owned(),
            host: "%".to_owned(),
            password: password.to_string(),
        },
    )
    .await?;
    Ok(())
}
