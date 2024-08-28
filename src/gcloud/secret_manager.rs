use crate::gcloud;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use log::info;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct AccessSecretVersion {
    name: String,
    payload: SecretPayload,
}

#[derive(Serialize, Deserialize, Debug)]
struct SecretPayload {
    data: String,
}

#[derive(Serialize, Debug, Default)]
struct CreateSecretRequest {
    replication: Replication,
    labels: HashMap<String, String>,
}

#[derive(Serialize, Debug, Default)]
struct Replication {
    automatic: Automatic,
}

#[derive(Serialize, Debug, Default)]
struct Automatic {}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct CreateSecretResponse {
    name: String,
}

#[derive(Serialize, Debug)]
struct AddSecretVersionRequest {
    payload: SecretPayload,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct SecretVersion {
    name: String,
}

pub async fn get_or_create(project: &str, name: &str, env: &str) -> String {
    let url = format!("https://secretmanager.googleapis.com/v1/projects/{project}/secrets/{name}/versions/latest:access");
    let response: Option<AccessSecretVersion> = gcloud::get(&url).await;

    match response {
        Some(version) => {
            let data = version.payload.data;
            let data = BASE64_STANDARD.decode(&data).expect("payload should be in base64 encoding");
            String::from_utf8(data).expect("data should be in utf-8")
        }
        None => {
            info!("secret not found, create new one, name={}", name);
            create(project, name, env).await;
            let value = Uuid::new_v4().to_string();
            add_secret_version(project, name, &value).await;
            value
        }
    }
}

async fn create(project: &str, name: &str, env: &str) {
    let url = format!("https://secretmanager.googleapis.com/v1/projects/{project}/secrets?secretId={name}");
    let mut create_secret_request = CreateSecretRequest::default();
    create_secret_request.labels.insert("env".to_string(), env.to_string());
    let _: CreateSecretResponse = gcloud::post(&url, &create_secret_request).await;
}

async fn add_secret_version(project: &str, name: &str, value: &str) {
    let url = format!("https://secretmanager.googleapis.com/v1/projects/{project}/secrets/{name}:addVersion");
    let add_secret_request = AddSecretVersionRequest {
        payload: SecretPayload {
            data: BASE64_STANDARD.encode(value),
        },
    };
    let _: SecretVersion = gcloud::post(&url, &add_secret_request).await;
}
