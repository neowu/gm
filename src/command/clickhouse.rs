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
    #[command(about = "show diff between clickhouse and config")]
    Status,
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
            Command::Status => status(&config).await,
            Command::SendPassword { user } => send_password(&config, user.as_deref()).await,
        }
    }
}

async fn connect(config: &ClickHouseConfig) -> ClickHouse {
    ClickHouse {
        url: config.url.clone(),
        user: "root".to_owned(),
        password: secret_manager::get(&config.project, &config.root_secret)
            .await
            .unwrap_or_else(|| panic!("root secret not found, secret={}", config.root_secret)),
    }
}

async fn sync(config: &ClickHouseConfig) {
    let clickhouse = connect(config).await;

    for profile in &config.profiles {
        let name = &profile.name;
        info!(profile = name, "sync profile");
        clickhouse.execute(&format!("CREATE SETTINGS PROFILE IF NOT EXISTS `{name}`")).await;
        // replace all settings and clear TO, keep profile id so assigned users stay linked (OR REPLACE creates new id)
        clickhouse
            .execute(&format!("ALTER SETTINGS PROFILE `{name}` SETTINGS {} TO NONE", profile.settings.join(", ")))
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

async fn status(config: &ClickHouseConfig) {
    let clickhouse = connect(config).await;

    let profiles = clickhouse.execute("SELECT name FROM system.settings_profiles FORMAT TSVRaw").await;
    for profile in &config.profiles {
        let name = &profile.name;
        if !profiles.lines().any(|p| p == name) {
            println!("profile {name}: not found");
            continue;
        }
        // format target by clickhouse, to compare with same normalization as SHOW CREATE
        let current = clickhouse.execute(&format!("SHOW CREATE SETTINGS PROFILE `{name}` FORMAT TSVRaw")).await;
        let target = format_query(
            &clickhouse,
            &format!("CREATE SETTINGS PROFILE `{name}` SETTINGS {}", profile.settings.join(", ")),
        )
        .await;
        print_diff(&format!("profile {name}"), &[current.trim_end().to_owned()], &[target]);
    }

    let users = clickhouse.execute("SELECT name FROM system.users FORMAT TSVRaw").await;
    for user in &config.users {
        let name = &user.name;
        if !users.lines().any(|u| u == name) {
            println!("user {name}: not found");
            continue;
        }
        let current: Vec<String> = clickhouse
            .execute(&format!("SHOW GRANTS FOR `{name}` FORMAT TSVRaw"))
            .await
            .lines()
            .flat_map(grant_elements)
            .collect();
        let mut target = vec![];
        for grant in &config.roles.iter().find(|r| r.name == user.role).expect("role should exist").grants {
            target.extend(grant_elements(&format_query(&clickhouse, &format!("GRANT {grant} TO `{name}`")).await));
        }
        print_diff(&format!("user {name}"), &current, &target);
    }
}

async fn format_query(clickhouse: &ClickHouse, query: &str) -> String {
    let literal = query.replace('\\', "\\\\").replace('\'', "\\'");
    clickhouse
        .execute(&format!("SELECT formatQuerySingleLine('{literal}') FORMAT TSVRaw"))
        .await
        .trim_end()
        .to_owned()
}

// split "GRANT A, B ON db.* TO user" into "A ON db.*", "B ON db.*", as clickhouse merges grants on same target
fn grant_elements(statement: &str) -> Vec<String> {
    let Some((body, to)) = statement.strip_prefix("GRANT ").and_then(|s| s.rsplit_once(" TO ")) else {
        return vec![statement.to_owned()];
    };
    let option = to.find(" WITH ").map(|i| &to[i..]).unwrap_or("");
    match body.rsplit_once(" ON ") {
        Some((privileges, target)) => split_top_level(privileges)
            .into_iter()
            .map(|privilege| format!("{privilege} ON {target}{option}"))
            .collect(),
        None => vec![format!("{body}{option}")],
    }
}

fn split_top_level(value: &str) -> Vec<String> {
    let mut items = vec![];
    let mut start = 0;
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (i, c) in value.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                items.push(value[start..i].trim().to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    items.push(value[start..].trim().to_owned());
    items
}

fn print_diff(title: &str, current: &[String], target: &[String]) {
    let removed: Vec<&String> = current.iter().filter(|c| !target.contains(c)).collect();
    let added: Vec<&String> = target.iter().filter(|t| !current.contains(t)).collect();
    if removed.is_empty() && added.is_empty() {
        println!("{title}: up to date");
        return;
    }
    println!("{title}:");
    for item in removed {
        println!("  - {item}");
    }
    for item in added {
        println!("  + {item}");
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
            "clickHouse password",
            &format!("请删除这封邮件，并妥善保管密码，谢谢\nuser: {}\npassword: {password}\n", user.name),
        )
        .await;
    }
}
