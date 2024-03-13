use crate::util::exception::Exception;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::OnceLock;

pub mod secret_manager;
pub mod sql_admin;

#[derive(Debug, Clone)]
pub struct NotFoundError {
    pub response: String,
}

impl NotFoundError {
    pub fn new(response: &str) -> NotFoundError {
        NotFoundError {
            response: response.to_string(),
        }
    }
}

impl Display for NotFoundError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.response)
    }
}

impl Error for NotFoundError {}

fn http_client() -> &'static reqwest::Client {
    static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(reqwest::Client::new)
}

pub(in crate::gcloud) async fn get<T>(url: &str) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    let token = env::var("GCLOUD_AUTH_TOKEN").expect("please set GCLOUD_AUTH_TOKEN env");

    let response = http_client()
        .get(url)
        .bearer_auth(token)
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;
    if status == 404 {
        return Err(Box::new(NotFoundError::new(&text)));
    }
    if status != 200 {
        return Err(Box::new(Exception::new(&format!(
            "failed to call api, status={}, response={}",
            status, text
        ))));
    }

    Ok(serde_json::from_str(&text)?)
}

pub(in crate::gcloud) async fn post<Request, Response>(url: &str, request: &Request) -> Result<Response, Box<dyn Error>>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    let token = env::var("GCLOUD_AUTH_TOKEN").expect("please set GCLOUD_AUTH_TOKEN env");

    let body = serde_json::to_string(request)?;
    let response = http_client()
        .post(url)
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;
    if status == 404 {
        return Err(Box::new(NotFoundError::new(&text)));
    }
    if status != 200 {
        return Err(Box::new(Exception::new(&format!(
            "failed to call api, status={}, response={}",
            status, text
        ))));
    }

    Ok(serde_json::from_str(&text)?)
}
