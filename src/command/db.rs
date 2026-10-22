use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use clap::Subcommand;
use tracing::info;

use crate::config::db_config::Auth;
use crate::config::db_config::DBConfig;
use crate::config::db_config::DBType;
use crate::db::Database;
use crate::gcloud::secret_manager;
use crate::gcloud::sql_admin;
use crate::kube;
use crate::util::json;

#[derive(Args)]
pub struct DBCommand {
    #[arg(long, global = true, help = "env path")]
    env: Option<PathBuf>,
    #[arg(long, global = true, help = "only apply this config file in env db dir, e.g. db.json")]
    config: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "sync db, users and kube endpoints")]
    Sync {
        #[arg(long, help = "revoke privileges of users not in config and drop PASSWORD ones, otherwise keep them")]
        force: bool,
    },
    #[command(about = "show diff between db and config")]
    Status,
}

impl DBCommand {
    pub async fn execute(&self) -> Result<()> {
        let env_dir = self.env.as_deref().unwrap_or(Path::new("."));
        if !env_dir.exists() {
            panic!("env dir doesn't exist, dir={}", env_dir.to_string_lossy());
        }
        let absolute_env_dir = fs::canonicalize(env_dir).unwrap_or_else(|err| panic!("{err}"));
        info!("env: {}", absolute_env_dir.to_string_lossy());

        let paths = db_config_paths(env_dir, self.config.as_deref());
        match &self.command {
            Command::Sync { force } => sync(env_dir, paths, *force).await,
            Command::Status => status(env_dir, paths).await,
        }
    }
}

async fn sync(env_dir: &Path, paths: Vec<PathBuf>, force: bool) -> Result<()> {
    for path in paths {
        info!("sync db config, config={}", path.to_string_lossy());
        let config = load_config(&path);
        let instance = sql_admin::get_sql_instance(&config.project, &config.instance).await;
        sync_db(&config, instance.public_address(), force).await?;
        sync_kube_endpoints(&config, env_dir, instance.private_address());
    }
    Ok(())
}

async fn status(env_dir: &Path, paths: Vec<PathBuf>) -> Result<()> {
    for path in paths {
        info!("load db config, config={}", path.to_string_lossy());
        let config = load_config(&path);
        println!("instance {}:", config.instance);

        let instance = sql_admin::get_sql_instance(&config.project, &config.instance).await;
        let root_password = secret_manager::get(&config.project, &config.root_secret)
            .await
            .unwrap_or_else(|| panic!("root secret not found, run sync first, secret={}", config.root_secret));
        let mut database = Database::create_database(&config.db_type, instance.public_address(), &root_password).await?;

        for db in &config.dbs {
            if database.db_exists(db).await? {
                println!("db {db}: up to date");
            } else {
                println!("db {db}: not found");
            }
        }

        for user in &config.users {
            let name = &user.name;
            match database.current_grants(user, &config.dbs).await? {
                Some(current) => print_diff(&format!("user {name}"), &current, &database.target_grants(user, &config.dbs)),
                None => println!("user {name}: not found"),
            }
        }
        for user in users_not_in_config(&config).await {
            println!("user {}: not in config", user.name);
        }

        let endpoint_path = env_dir.join(&config.endpoint.path);
        let target = kube::endpoint::Endpoint {
            name: &config.endpoint.name,
            ns: &config.endpoint.ns,
            ip: instance.private_address(),
        }
        .to_kube_config();
        match fs::read_to_string(&endpoint_path) {
            Ok(current) if current == target => println!("endpoint {}: up to date", config.endpoint.path),
            Ok(_) => println!("endpoint {}: outdated", config.endpoint.path),
            Err(_) => println!("endpoint {}: not found", config.endpoint.path),
        }
    }
    Ok(())
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

fn load_config(path: &Path) -> DBConfig {
    let content = fs::read_to_string(path).unwrap_or_else(|err| panic!("{err}"));
    let config: DBConfig = json::from_json(&content);
    config.validate();
    config
}

async fn sync_db(config: &DBConfig, public_ip: &str, force: bool) -> Result<()> {
    let root_user = root_user(&config.db_type);
    let root_password = secret_manager::get_or_create(&config.project, &config.root_secret, &config.env).await;
    sql_admin::set_password(&config.project, &config.instance, root_user, &root_password).await;

    let mut database = Database::create_database(&config.db_type, public_ip, &root_password).await?;

    for db in &config.dbs {
        database.create_db(db).await?;
    }

    for user in &config.users {
        if let Auth::Password = user.auth {
            let password = secret_manager::get_or_create(&config.project, user.secret.as_ref().unwrap(), &config.env).await;
            database.create_user(user, &password).await?;
        }
        database.grant_user_privileges(user, &config.dbs).await?;
    }

    for user in users_not_in_config(config).await {
        let name = &user.name;
        if !force {
            info!(user = name, "keep user not in config, use --force to revoke");
            continue;
        }
        database.revoke_user_privileges(name, &config.dbs).await?;
        if user.iam() {
            // IAM user is created by gcloud, so delete it by gcloud as well (e.g. terraform)
            info!(user = name, "revoked IAM user privileges, delete user by gcloud");
        } else {
            database.drop_user(name).await?;
        }
    }

    Ok(())
}

async fn users_not_in_config(config: &DBConfig) -> Vec<sql_admin::SQLUser> {
    let root_user = root_user(&config.db_type);
    sql_admin::list_users(&config.project, &config.instance)
        .await
        .into_iter()
        .filter(|user| user.name != root_user && !config.users.iter().any(|u| u.name == user.name))
        .collect()
}

fn root_user(db_type: &DBType) -> &'static str {
    match db_type {
        DBType::MySQL => "root",
        DBType::PostgreSQL => "postgres",
    }
}

fn sync_kube_endpoints(config: &DBConfig, env_dir: &Path, private_ip: &str) {
    let endpoint_path = env_dir.join(&config.endpoint.path);
    info!(path = endpoint_path.to_str(), "write kube endpoint");
    fs::create_dir_all(endpoint_path.parent().expect("endpoint should have parent dir")).unwrap_or_else(|err| panic!("{err}"));

    let contents = kube::endpoint::Endpoint {
        name: &config.endpoint.name,
        ns: &config.endpoint.ns,
        ip: private_ip,
    }
    .to_kube_config();
    fs::write(endpoint_path, contents).unwrap_or_else(|err| panic!("{err}"));
}

fn db_config_paths(env_dir: &Path, config: Option<&str>) -> Vec<PathBuf> {
    let db_dir = env_dir.join("db");

    if !db_dir.exists() {
        panic!("db dir doesn't exist, dir={}", db_dir.to_string_lossy());
    }

    if let Some(config) = config {
        let path = db_dir.join(config);
        if !path.is_file() {
            panic!("db config doesn't exist, config={}", path.to_string_lossy());
        }
        return vec![path];
    }

    let paths: Vec<PathBuf> = fs::read_dir(&db_dir)
        .unwrap_or_else(|err| panic!("{err}"))
        .flatten()
        .filter(|entry| {
            if let Some(file_name) = entry.file_name().to_str() {
                return file_name.ends_with(".json");
            }
            false
        })
        .map(|entry| entry.path())
        .collect();

    paths
}
