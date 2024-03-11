use std::error::Error;
use std::fs::{self};
use std::path::PathBuf;

use clap::Args;

use crate::command::db_config::DBConfig;
use crate::gcloud::{secret_manager, sql_admin};
use crate::mysql::MySQLClient;
use crate::util::json;

#[derive(Args)]
#[command(about = "Sync db")]
pub struct SyncDB {
    #[arg(long, help = "conf path")]
    conf: Option<String>,
}

impl SyncDB {
    pub async fn execute(&self) -> Result<(), Box<dyn Error>> {
        let paths = self.db_config_paths()?;

        for path in paths {
            let content = fs::read_to_string(path)?;
            let config: DBConfig = json::from_json(&content)?;
            config.validate()?;

            self.sync(config).await?;
        }

        Ok(())
    }

    async fn sync(&self, config: DBConfig) -> Result<(), Box<dyn Error>> {
        let instance = sql_admin::get_sql_instance(&config.project, &config.instance).await?;
        let public_ip = instance.public_address().expect("public ip must not be null");
        let root_password = secret_manager::get_or_create(&config.project, &config.root_secret, &config.env).await?;
        sql_admin::set_root_password(&config.project, &config.instance, &root_password).await?;

        let mut mysql = MySQLClient::new(public_ip, "root", &root_password)?;

        for user in &config.users {
            match user.auth {
                crate::command::db_config::Auth::IAM => mysql.grant_user_privileges(&user.name, &config.dbs(user), &user.privileges())?,
                crate::command::db_config::Auth::PASSWORD => {
                    let password = secret_manager::get_or_create(&config.project, user.secret.as_ref().unwrap().as_str(), &config.env).await?;
                    mysql.create_user(&user.name, &password)?;
                    mysql.grant_user_privileges(&user.name, &config.dbs(user), &user.privileges())?
                }
            }
        }

        Ok(())
    }

    fn db_config_paths(&self) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let db_dir = self.conf.as_ref().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("db"));

        if !db_dir.exists() {
            return Err((format!("db dir doesn't exist, dir={}", db_dir.to_string_lossy())).into());
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
}
