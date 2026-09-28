use std::error::Error;

use crate::util::http_client::HTTP_CLIENT;

pub struct ClickHouse {
    pub url: String,
    pub user: String,
    pub password: String,
}

impl ClickHouse {
    pub async fn execute(&self, statement: &str) -> String {
        let response = HTTP_CLIENT
            .post(&self.url)
            .header("X-ClickHouse-User", &self.user)
            .header("X-ClickHouse-Key", &self.password)
            .body(statement.to_owned())
            .send()
            .await
            .unwrap_or_else(|err| panic!("{err}, source={:?}", err.source()));

        let status = response.status();
        let text = response.text().await.unwrap_or_else(|err| panic!("{err}"));
        if status != 200 {
            panic!("failed to execute clickhouse statement, status={status}, response={text}");
        }
        text
    }
}
