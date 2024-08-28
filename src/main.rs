use clap::Parser;
use clap::Subcommand;
use command::completion::Completion;
use command::sync_db::SyncDB;

mod command;
mod config;
mod gcloud;
mod kube;
mod mysql;
mod util;

#[derive(Parser)]
#[command(author, version)]
#[command(about = "gcloud manager cli")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(arg_required_else_help(true))]
pub enum Commands {
    #[command(about = "sync db")]
    DB(SyncDB),
    #[command(about = "generate shell completion")]
    Completion(Completion),
}

#[tokio::main]
async fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Info).init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::DB(command) => command.execute().await,
        Commands::Completion(command) => command.execute(),
    }
}
