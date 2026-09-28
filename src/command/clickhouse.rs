use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;
use tracing::info;

use crate::clickhouse::ClickHouse;
use crate::config::clickhouse_config::ClickHouseConfig;
use crate::config::clickhouse_config::User;
use crate::gcloud::secret_manager;
use crate::sendgrid;

#[derive(Args)]
pub struct ClickHouseCommand {
    #[arg(long, global = true, help = "env path")]
    env: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "sync clickhouse profiles and users")]
    Sync,
    #[command(about = "send password email to users")]
    SendPassword {
        #[arg(long, help = "user name, default to all users with email")]
        user: Option<String>,
    },
}

impl ClickHouseCommand {
    pub async fn execute(&self) {
        let env_dir = self.env.as_deref().unwrap_or(Path::new("."));
        let path = env_dir.join("clickhouse/clickhouse.jsonc");
        info!("load clickhouse config, config={}", path.to_string_lossy());
        let content = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{err}, path={}", path.to_string_lossy()));
        let config: ClickHouseConfig = json5::from_str(&content).unwrap_or_else(|err| panic!("failed to parse config, error={err}"));
        config.validate();

        match &self.command {
            Command::Sync => sync(&config).await,
            Command::SendPassword { user } => send_password(&config, user.as_deref()).await,
        }
    }
}

async fn sync(config: &ClickHouseConfig) {
    let clickhouse = ClickHouse {
        url: config.url.clone(),
        user: "root".to_owned(),
        password: secret_manager::get(&config.project, &config.root_secret)
            .await
            .unwrap_or_else(|| panic!("root secret not found, secret={}", config.root_secret)),
    };

    for profile in &config.profiles {
        let name = &profile.name;
        info!(profile = name, "sync profile");
        clickhouse.execute(&format!("CREATE SETTINGS PROFILE IF NOT EXISTS `{name}`")).await;
        // bare SETTINGS replaces all existing settings of profile
        clickhouse
            .execute(&format!("ALTER SETTINGS PROFILE `{name}` SETTINGS {}", profile.settings.join(", ")))
            .await;
    }

    for user in &config.users {
        let name = &user.name;
        info!(user = name, "sync user");
        let password = secret_manager::get_or_create(&config.project, &user.secret, &config.env).await;
        clickhouse
            .execute(&format!("CREATE USER IF NOT EXISTS `{name}` IDENTIFIED BY '{password}'"))
            .await;
        clickhouse.execute(&format!("ALTER USER `{name}` IDENTIFIED BY '{password}'")).await;

        let profile = &user.profile;
        info!(user = name, profile, "assign profile");
        clickhouse.execute(&format!("ALTER USER `{name}` SETTINGS PROFILE '{profile}'")).await;

        for grant in &config.roles.iter().find(|r| r.name == user.role).expect("role should exist").grants {
            info!(user = name, grant, "grant");
            clickhouse.execute(&format!("GRANT {grant} TO `{name}`")).await;
        }
    }
}

async fn send_password(config: &ClickHouseConfig, user_name: Option<&str>) {
    let users: Vec<&User> = match user_name {
        Some(name) => {
            let user = config
                .users
                .iter()
                .find(|u| u.name == name)
                .unwrap_or_else(|| panic!("user not found, user={name}"));
            if user.email.is_none() {
                panic!("user doesn't have email, user={name}");
            }
            vec![user]
        }
        None => config.users.iter().filter(|u| u.email.is_some()).collect(),
    };

    for user in users {
        let email = user.email.as_deref().unwrap();
        info!(user = user.name, email, "send password");
        let password = secret_manager::get(&config.project, &user.secret)
            .await
            .unwrap_or_else(|| panic!("user secret not found, run sync first, user={}, secret={}", user.name, user.secret));
        sendgrid::send(
            &config.email.token,
            &config.email.from_address,
            email,
            "ClickHouse password",
            &format!("user: {}\npassword: {password}\n", user.name),
        )
        .await;
    }
}
