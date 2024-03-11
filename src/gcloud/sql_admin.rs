use super::client;
use serde::{Deserialize, Serialize};
use std::error::Error;

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

#[derive(Deserialize, Debug)]
struct Operation {
    kind: String,
}

impl GetSQLInstanceResponse {
    pub fn public_address(&self) -> Option<&String> {
        self.addresses.iter().find(|ip| ip.r#type == "PRIMARY").map(|ip| &ip.address)
    }
}

pub async fn get_sql_instance(project: &str, instance: &str) -> Result<GetSQLInstanceResponse, Box<dyn Error>> {
    let url = format!("https://sqladmin.googleapis.com/v1/projects/{project}/instances/{instance}");
    client::get(&url).await
}

pub async fn set_root_password(project: &str, instance: &str, password: &str) -> Result<(), Box<dyn Error>> {
    println!("change sql instance root password, instance={instance}");
    let url = format!("https://sqladmin.googleapis.com/v1/projects/{project}/instances/{instance}/users");
    let _: Operation = client::post(
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
