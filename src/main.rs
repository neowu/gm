use clap::Parser;
use clap::Subcommand;
use command::generate_zsh_completion::GenerateZshCompletion;
use command::sync_db::SyncDB;
use std::error::Error;
use tracing::Level;
use util::exception::Exception;

mod command;
mod gcloud;
mod kube;
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
async fn main() -> Result<(), Exception> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).with_line_number(true).init();

    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::DB(command)) => command.execute().await,
        Some(Commands::GenerateZshCompletion(command)) => command.execute(),
        None => panic!("not implemented"),
    }
}
