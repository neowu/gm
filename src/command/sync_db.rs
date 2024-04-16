use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Args;
use std::io::Error;
use tracing::info;

use crate::command::db_config::DBConfig;
use crate::gcloud::secret_manager;
use crate::gcloud::sql_admin;
use crate::kube;
use crate::mysql::MySQLClient;
use crate::util::exception::Exception;
use crate::util::json;

#[derive(Args)]
#[command(about = "Sync db")]
pub struct SyncDB {
    #[arg(long, help = "env path")]
    env: Option<String>,
}

impl SyncDB {
    pub async fn execute(&self) -> Result<(), Exception> {
        let env_dir = self.env.as_ref().map(Path::new).unwrap_or_else(|| Path::new("."));
        if !env_dir.exists() {
            return Err(Exception::new(&format!("env dir doesn't exist, dir={}", env_dir.to_string_lossy())));
        }
        info!("env: {}", fs::canonicalize(env_dir).unwrap().to_string_lossy());

        let paths = db_config_paths(env_dir)?;

        for path in paths {
            info!("sync db config, config={}", path.to_string_lossy());
            let content = fs::read_to_string(path)?;
            let config: DBConfig = json::from_json(&content)?;
            config.validate()?;

            let instance = sql_admin::get_sql_instance(&config.project, &config.instance)
                .await
                .map_err(Exception::from)?;
            let public_ip = instance.public_address()?;
            let private_ip = instance.private_address()?;
            sync_users(&config, public_ip).await?;
            sync_kube_endpoints(&config, env_dir, private_ip)?;
        }

        Ok(())
    }
}

async fn sync_users(config: &DBConfig, public_ip: &str) -> Result<(), Exception> {
    let root_password = secret_manager::get_or_create(&config.project, &config.root_secret, &config.env)
        .await
        .map_err(Exception::from)?;
    sql_admin::set_root_password(&config.project, &config.instance, &root_password)
        .await
        .map_err(Exception::from)?;

    let mut mysql = MySQLClient::new(public_ip, "root", &root_password)?;

    for user in &config.users {
        match user.auth {
            crate::command::db_config::Auth::Iam => mysql.grant_user_privileges(&user.name, &config.dbs(user), &user.privileges())?,
            crate::command::db_config::Auth::Password => {
                let password = secret_manager::get_or_create(&config.project, user.secret.as_ref().unwrap(), &config.env)
                    .await
                    .map_err(Exception::from)?;
                mysql.create_user(&user.name, &password)?;
                mysql.grant_user_privileges(&user.name, &config.dbs(user), &user.privileges())?
            }
        }
    }

    Ok(())
}

fn sync_kube_endpoints(config: &DBConfig, env_dir: &Path, private_ip: &str) -> Result<(), Exception> {
    for endpoint in &config.endpoints {
        let endpoint_path = env_dir.join(&endpoint.path);
        fs::create_dir_all(endpoint_path.parent().ok_or(Exception::new("endpoint path must have parent dir"))?)?;

        let content = kube::endpoint::Endpoint {
            name: &endpoint.name,
            ns: &endpoint.ns,
            ip: private_ip,
        }
        .to_resource();
        fs::write(endpoint_path, content)?
    }
    Ok(())
}

fn db_config_paths(env_dir: &Path) -> Result<Vec<PathBuf>, Exception> {
    let db_dir = env_dir.join("db");

    if !db_dir.exists() {
        return Err(Exception::new(&format!("db dir doesn't exist, dir={}", db_dir.to_string_lossy())));
    }

    let paths: Vec<PathBuf> = fs::read_dir(&db_dir)?
        .flatten()
        .filter(|entry| {
            if let Some(file_name) = entry.file_name().to_str() {
                return file_name.ends_with(".json");
            }
            false
        })
        .map(|entry| entry.path())
        .collect();

    Ok(paths)
}
