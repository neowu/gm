use std::error::Error;
use std::io;

use clap::Args;
use clap::CommandFactory;
use clap_complete::generate;
use clap_complete::shells::Zsh;

use crate::util::exception::Exception;
use crate::Cli;

const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Args)]
#[command(about = "Generate zsh completion")]
pub struct GenerateZshCompletion {}

impl GenerateZshCompletion {
    pub fn execute(&self) -> Result<(), Exception> {
        generate(Zsh, &mut Cli::command(), CARGO_PKG_NAME, &mut io::stdout());
        Ok(())
    }
}
