use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ClickHouseConfig {
    pub version: String,
    pub project: String,
    pub env: String,
    pub url: String,
    #[serde(rename(deserialize = "rootSecret"))]
    pub root_secret: String,
    pub email: Email,
    pub profiles: Vec<Profile>,
    pub roles: Vec<Role>,
    pub users: Vec<User>,
}

#[derive(Deserialize, Debug)]
pub struct Email {
    pub token: String,
    #[serde(rename(deserialize = "fromAddress"))]
    pub from_address: String,
}

#[derive(Deserialize, Debug)]
pub struct Profile {
    pub name: String,
    pub settings: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Role {
    pub name: String,
    pub grants: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub name: String,
    pub secret: String,
    pub profile: String,
    pub role: String,
    pub email: Option<String>,
}

impl ClickHouseConfig {
    pub fn validate(&self) {
        let version = env!("CARGO_PKG_VERSION");
        let config_version = &self.version;
        if config_version != version {
            panic!("config version does not match gm, config_version={config_version}, gm_version={version}");
        }

        for user in &self.users {
            if !self.profiles.iter().any(|p| p.name == user.profile) {
                panic!("user profile not found, user={}, profile={}", user.name, user.profile);
            }
            if !self.roles.iter().any(|r| r.name == user.role) {
                panic!("user role not found, user={}, role={}", user.name, user.role);
            }
        }
    }
}
