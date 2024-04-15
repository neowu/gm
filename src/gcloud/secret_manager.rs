use crate::gcloud;
use crate::util::exception::Exception;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::GCloudError;

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

pub async fn get_or_create(project: &str, name: &str, env: &str) -> Result<String, GCloudError> {
    let url = format!("https://secretmanager.googleapis.com/v1/projects/{project}/secrets/{name}/versions/latest:access");
    let response: Result<AccessSecretVersion, GCloudError> = gcloud::get(&url).await;

    match response {
        Ok(version) => {
            let data = BASE64_STANDARD
                .decode(version.payload.data)
                .map_err(|err| GCloudError::Other(Exception::new(&err.to_string())))?;
            Ok(String::from_utf8(data).map_err(|err| GCloudError::Other(Exception::new(&err.to_string())))?)
        }
        Err(GCloudError::NotFound { response: _ }) => {
            println!("secret not found, create new one, name={}", name);
            create(project, name, env).await?;
            let value = Uuid::new_v4().to_string();
            add_secret_version(project, name, &value).await?;
            Ok(value)
        }
        Err(error) => Err(error),
    }
}

pub async fn create(project: &str, name: &str, env: &str) -> Result<(), GCloudError> {
    let url = format!("https://secretmanager.googleapis.com/v1/projects/{project}/secrets?secretId={name}");
    let mut create_secret_request = CreateSecretRequest::default();
    create_secret_request.labels.insert("env".to_owned(), env.to_string());
    let _: CreateSecretResponse = gcloud::post(&url, &create_secret_request).await?;
    Ok(())
}

pub async fn add_secret_version(project: &str, name: &str, value: &str) -> Result<(), GCloudError> {
    let url = format!("https://secretmanager.googleapis.com/v1/projects/{project}/secrets/{name}:addVersion");
    let add_secret_request = AddSecretVersionRequest {
        payload: SecretPayload {
            data: BASE64_STANDARD.encode(value),
        },
    };
    let _: SecretVersion = gcloud::post(&url, &add_secret_request).await?;
    Ok(())
}
