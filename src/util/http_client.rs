use std::sync::LazyLock;

pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
