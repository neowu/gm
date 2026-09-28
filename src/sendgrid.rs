use std::error::Error;

use serde::Serialize;

use crate::util::http_client::HTTP_CLIENT;
use crate::util::json;

#[derive(Serialize, Debug)]
struct MailRequest<'a> {
    personalizations: [Personalization<'a>; 1],
    from: Address<'a>,
    subject: &'a str,
    content: [Content<'a>; 1],
}

#[derive(Serialize, Debug)]
struct Personalization<'a> {
    to: [Address<'a>; 1],
}

#[derive(Serialize, Debug)]
struct Address<'a> {
    email: &'a str,
}

#[derive(Serialize, Debug)]
struct Content<'a> {
    #[serde(rename = "type")]
    content_type: &'a str,
    value: &'a str,
}

pub async fn send(token: &str, from: &str, to: &str, subject: &str, content: &str) {
    let request = MailRequest {
        personalizations: [Personalization { to: [Address { email: to }] }],
        from: Address { email: from },
        subject,
        content: [Content {
            content_type: "text/plain",
            value: content,
        }],
    };
    let response = HTTP_CLIENT
        .post("https://api.sendgrid.com/v3/mail/send")
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .body(json::to_json(&request))
        .send()
        .await
        .unwrap_or_else(|err| panic!("{err}, source={:?}", err.source()));

    let status = response.status();
    if status != 202 {
        let text = response.text().await.unwrap_or_else(|err| panic!("{err}"));
        panic!("failed to send email, status={status}, response={text}");
    }
}
