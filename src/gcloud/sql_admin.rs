use std::{env, error::Error};

use serde::Deserialize;

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
        self.addresses
            .iter()
            .find(|ip| ip.r#type == "PRIMARY")
            .map(|ip| &ip.address)
    }
}

pub async fn get_sql_instance(project: &str, instance: &str) -> Result<GetSQLInstanceResponse, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!("https://sqladmin.googleapis.com/v1/projects/{project}/instances/{instance}");
    let token = env::var("GCLOUD_AUTH_TOKEN").expect("please set GCLOUD_AUTH_TOKEN env");
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;
    let text = response.text().await?;
    Ok(serde_json::from_str(&text)?)
}
