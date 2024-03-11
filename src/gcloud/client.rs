use std::{
    env,
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::util::exception::Exception;

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

pub async fn get<T>(url: &str) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    let client = reqwest::Client::new();
    let token = env::var("GCLOUD_AUTH_TOKEN").expect("please set GCLOUD_AUTH_TOKEN env");

    let response = client.get(url).bearer_auth(token).header("Accept", "application/json").send().await?;

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

pub async fn post<Request, Response>(url: &str, request: &Request) -> Result<Response, Box<dyn Error>>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    let client = reqwest::Client::new();
    let token = env::var("GCLOUD_AUTH_TOKEN").expect("please set GCLOUD_AUTH_TOKEN env");

    let body = serde_json::to_string(request)?;
    let response = client
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
