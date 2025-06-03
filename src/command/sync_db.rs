use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use tracing::info;

use crate::config::db_config::Auth;
use crate::config::db_config::DBConfig;
use crate::db::Database;
use crate::gcloud::secret_manager;
use crate::gcloud::sql_admin;
use crate::kube;
use crate::util::json;

#[derive(Args)]
pub struct SyncDB {
    #[arg(long, help = "env path")]
    env: Option<PathBuf>,
}

impl SyncDB {
    pub async fn execute(&self) -> Result<()> {
        rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();

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
            sync_db(&config, public_ip).await?;
            sync_kube_endpoints(&config, env_dir, private_ip);
        }

        Ok(())
    }
}

async fn sync_db(config: &DBConfig, public_ip: &str) -> Result<()> {
    let root_password = secret_manager::get_or_create(&config.project, &config.root_secret, &config.env).await;
    sql_admin::set_root_password(&config.project, &config.instance, &root_password).await;

    let mut database = Database::create_database(&config.db_type, public_ip, &root_password).await?;

    for db in &config.dbs {
        database.create_db(db).await?;
    }

    for user in &config.users {
        if let Auth::Password = user.auth {
            let password = secret_manager::get_or_create(&config.project, user.secret.as_ref().unwrap(), &config.env).await;
            database.create_user(&user.name, &password).await?;
        }
        database.grant_user_privileges(user, &config.dbs).await?;
    }

    Ok(())
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
