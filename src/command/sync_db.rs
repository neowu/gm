use std::error::Error;
use std::fs::{self};
use std::path::PathBuf;

use clap::Args;

use crate::command::db_config::DBConfig;

#[derive(Args)]
#[command(about = "Sync db")]
pub struct SyncDB {
    #[arg(long, help = "conf path")]
    conf: Option<String>,
}

impl SyncDB {
    pub fn execute(&self) -> Result<(), Box<dyn Error>> {
        let paths = self.db_config_paths()?;
        let configs: Vec<DBConfig> = paths
            .iter()
            .map(|path| {
                fs::read_to_string(path)
                    .unwrap_or_else(|err| panic!("failed to read file, err={}, path={}", err, path.to_string_lossy()))
            })
            .map(|content| {
                serde_json::from_str(&content)
                    .unwrap_or_else(|err| panic!("failed to deserialize config, err={}, content={}", err, content))
            })
            .collect();
        println!("{:?}", configs);
        Ok(())
    }

    fn db_config_paths(&self) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        let db_dir = self
            .conf
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("db"));

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
