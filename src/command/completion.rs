use std::io;

use clap::Args;
use clap::CommandFactory;
use clap_complete::generate;
use clap_complete::Shell;

use crate::Cli;

const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Args)]
pub struct Completion;

impl Completion {
    pub fn execute(&self) {
        let shell = Shell::from_env().expect("unknown shell");
        generate(shell, &mut Cli::command(), CARGO_PKG_NAME, &mut io::stdout());
    }
}
