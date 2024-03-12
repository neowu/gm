use clap::{Parser, Subcommand};
use command::{generate_zsh_completion::GenerateZshCompletion, sync_db::SyncDB};
use std::error::Error;

mod command;
mod gcloud;
mod mysql;
mod util;

#[derive(Parser)]
#[command(author, version)]
#[command(about = "GCloud manager cli")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
#[command(arg_required_else_help(true))]
pub enum Commands {
    DB(SyncDB),
    GenerateZshCompletion(GenerateZshCompletion),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::DB(command)) => command.execute().await,
        Some(Commands::GenerateZshCompletion(command)) => command.execute(),
        None => panic!("not implemented"),
    }
}
