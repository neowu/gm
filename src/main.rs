use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use command::completion::Completion;
use command::sync_db::SyncDB;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod command;
mod config;
mod db;
mod gcloud;
mod kube;
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
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_line_number(true)
                .with_thread_ids(true)
                .with_filter(LevelFilter::INFO),
        )
        .init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::DB(command) => command.execute().await?,
        Commands::Completion(command) => command.execute(),
    }
    Ok(())
}
