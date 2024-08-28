use crate::util::http_client::HTTP_CLIENT;
use crate::util::json;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env;
use std::fmt::Debug;

pub mod secret_manager;
pub mod sql_admin;

pub(in crate::gcloud) async fn get<Response>(url: &str) -> Option<Response>
where
    Response: DeserializeOwned,
{
    let response = HTTP_CLIENT
        .get(url)
        .bearer_auth(token())
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap_or_else(|err| panic!("{err}"));

    let status = response.status();
    let text = response.text().await.unwrap_or_else(|err| panic!("{err}"));
    if status == 404 {
        return None;
    }
    if status != 200 {
        panic!("failed to call api, status={status}, response={text}");
    }
    Some(json::from_json(&text))
}

pub(in crate::gcloud) async fn post<Request, Response>(url: &str, request: &Request) -> Response
where
    Request: Serialize + Debug,
    Response: DeserializeOwned,
{
    let body = json::to_json(request);
    let response = HTTP_CLIENT
        .post(url)
        .bearer_auth(token())
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .unwrap_or_else(|err| panic!("{err}"));

    let status = response.status();
    let text = response.text().await.unwrap_or_else(|err| panic!("{err}"));
    if status != 200 {
        panic!("failed to call api, status={status}, response={text}");
    }
    json::from_json(&text)
}

fn token() -> String {
    env::var("GCLOUD_AUTH_TOKEN").expect("please set GCLOUD_AUTH_TOKEN env")
}
