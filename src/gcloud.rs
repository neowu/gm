use crate::util::exception::Exception;
use crate::util::json;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env;
use std::error::Error;
use std::fmt::{self, Debug};
use std::sync::OnceLock;

pub mod secret_manager;
pub mod sql_admin;

#[derive(Debug)]
pub enum GCloudError {
    NotFound { response: String },
    Other(Exception),
}

impl fmt::Display for GCloudError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<reqwest::Error> for GCloudError {
    fn from(err: reqwest::Error) -> Self {
        GCloudError::Other(Exception::new(&err.to_string()))
    }
}

impl From<Exception> for GCloudError {
    fn from(err: Exception) -> Self {
        GCloudError::Other(err)
    }
}

impl Error for GCloudError {}

fn http_client() -> &'static reqwest::Client {
    static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(reqwest::Client::new)
}

pub(in crate::gcloud) async fn get<T>(url: &str) -> Result<T, GCloudError>
where
    T: DeserializeOwned,
{
    let response = http_client()
        .get(url)
        .bearer_auth(token())
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;
    if let Some(err) = validate(status, &text) {
        return Err(err);
    }
    Ok(json::from_json(&text)?)
}

pub(in crate::gcloud) async fn post<Request, Response>(url: &str, request: &Request) -> Result<Response, GCloudError>
where
    Request: Serialize + Debug,
    Response: DeserializeOwned,
{
    let body = json::to_json(request)?;
    let response = http_client()
        .post(url)
        .bearer_auth(token())
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;
    if let Some(err) = validate(status, &text) {
        return Err(err);
    }
    Ok(json::from_json(&text)?)
}

fn validate(status: reqwest::StatusCode, text: &String) -> Option<GCloudError> {
    if status == 404 {
        return Some(GCloudError::NotFound { response: text.clone() });
    }
    if status != 200 {
        return Some(GCloudError::Other(Exception::new(&format!(
            "failed to call api, status={}, response={}",
            status, text
        ))));
    }
    None
}

fn token() -> String {
    env::var("GCLOUD_AUTH_TOKEN").expect("please set GCLOUD_AUTH_TOKEN env")
}
