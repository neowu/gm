use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Args;

use log::info;

use crate::config::db_config;
use crate::config::db_config::DBConfig;
use crate::gcloud::secret_manager;
use crate::gcloud::sql_admin;
use crate::kube;
use crate::mysql::MySQLClient;
use crate::util::json;

#[derive(Args)]
pub struct SyncDB {
    #[arg(long, help = "env path")]
    env: Option<PathBuf>,
}

impl SyncDB {
    pub async fn execute(&self) {
        let env_dir = self.env.as_deref().unwrap_or(Path::new("."));
        if !env_dir.exists() {
            panic!("env dir doesn't exist, dir={}", env_dir.to_string_lossy());
        }
        let absolute_env_dir = fs::canonicalize(env_dir).unwrap_or_else(|err| panic!("{err}"));
        info!("env: {}", absolute_env_dir.to_string_lossy());

        let paths = db_config_paths(env_dir);

        for path in paths {
            info!("sync db config, config={}", path.to_string_lossy());
            let content = fs::read_to_string(path).unwrap_or_else(|err| panic!("{err}"));
            let config: DBConfig = json::from_json(&content);
            config.validate();

            let instance = sql_admin::get_sql_instance(&config.project, &config.instance).await;
            let public_ip = instance.public_address();
            let private_ip = instance.private_address();
            sync_users(&config, public_ip).await;
            sync_kube_endpoints(&config, env_dir, private_ip);
        }
    }
}

async fn sync_users(config: &DBConfig, public_ip: &str) {
    let root_password = secret_manager::get_or_create(&config.project, &config.root_secret, &config.env).await;
    sql_admin::set_root_password(&config.project, &config.instance, &root_password).await;

    let mut mysql = MySQLClient::new(public_ip, "root", &root_password);

    for user in &config.users {
        match user.auth {
            db_config::Auth::Iam => mysql.grant_user_privileges(&user.name, &config.dbs(user), &user.privileges()),
            db_config::Auth::Password => {
                let password = secret_manager::get_or_create(&config.project, user.secret.as_ref().unwrap(), &config.env).await;
                mysql.create_user(&user.name, &password);
                mysql.grant_user_privileges(&user.name, &config.dbs(user), &user.privileges())
            }
        }
    }
}

fn sync_kube_endpoints(config: &DBConfig, env_dir: &Path, private_ip: &str) {
    for endpoint in &config.endpoints {
        let endpoint_path = env_dir.join(&endpoint.path);
        fs::create_dir_all(endpoint_path.parent().expect("endpoint should have parent dir")).unwrap_or_else(|err| panic!("{err}"));

        let content = kube::endpoint::Endpoint {
            name: &endpoint.name,
            ns: &endpoint.ns,
            ip: private_ip,
        }
        .to_resource();
        fs::write(endpoint_path, content).unwrap_or_else(|err| panic!("{err}"));
    }
}

fn db_config_paths(env_dir: &Path) -> Vec<PathBuf> {
    let db_dir = env_dir.join("db");

    if !db_dir.exists() {
        panic!("db dir doesn't exist, dir={}", db_dir.to_string_lossy());
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
