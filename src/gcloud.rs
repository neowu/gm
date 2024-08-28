use crate::util::http_client::HTTP_CLIENT;
use crate::util::json;
use log::info;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env;
use std::error::Error;
use std::fmt::Debug;
use std::ops::Deref;
use std::process::Command;
use std::process::Stdio;
use std::sync::LazyLock;

pub mod secret_manager;
pub mod sql_admin;

pub(in crate::gcloud) async fn get<Response>(url: &str) -> Option<Response>
where
    Response: DeserializeOwned,
{
    let response = HTTP_CLIENT
        .get(url)
        .bearer_auth(TOKEN.deref())
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap_or_else(|err| panic!("{err}, source={:?}", err.source()));

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
        .bearer_auth(TOKEN.deref())
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .unwrap_or_else(|err| panic!("{err}, source={:?}", err.source()));

    let status = response.status();
    let text = response.text().await.unwrap_or_else(|err| panic!("{err}"));
    if status != 200 {
        panic!("failed to call api, status={status}, response={text}");
    }
    json::from_json(&text)
}

static TOKEN: LazyLock<String> = LazyLock::new(|| {
    let token = env::var("GCLOUD_AUTH_TOKEN");
    if let Ok(token) = token {
        info!("auth gcloud via GCLOUD_AUTH_TOKEN env");
        return token;
    }

    info!("auth gcloud via gcloud auth print-access-token");
    let output = Command::new("gcloud")
        .args(["auth", "print-access-token"])
        .stdout(Stdio::piped())
        .output()
        .unwrap_or_else(|err| panic!("please setup gcloud or set GCLOUD_AUTH_TOKEN env, err={err}"));

    let token = String::from_utf8(output.stdout).expect("token should be in utf-8");
    // print token contains '\n' at ends
    token.trim_ascii_end().to_string()
});
