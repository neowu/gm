use super::client;
use serde::Deserialize;
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

impl GetSQLInstanceResponse {
    pub fn public_address(&self) -> Option<&String> {
        self.addresses.iter().find(|ip| ip.r#type == "PRIMARY").map(|ip| &ip.address)
    }
}

pub async fn get_sql_instance(project: &str, instance: &str) -> Result<GetSQLInstanceResponse, Box<dyn Error>> {
    let url = format!("https://sqladmin.googleapis.com/v1/projects/{project}/instances/{instance}");
    client::get(&url).await
}
