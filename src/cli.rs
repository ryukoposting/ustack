use clap::{Parser, Subcommand};
use log::LevelFilter;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = "Copyright (c) 2023 Evan Grove")]
pub struct Cli {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Debug, Subcommand)]
pub enum Action {
    /// Start the HTTP server.
    Serve(crate::serve::Serve),
    /// Initialize a new blog in the current working directory.
    Init(crate::init::Init),
    /// Generate new things from a template.
    Generate(crate::generate::Generate)
}

impl Action {
    pub fn log_level(&self) -> LevelFilter {
        match self {
            Action::Serve(serve) => serve.log_level,
            Action::Init(init) => init.log_level,
            Action::Generate(generate) => generate.log_level,
        }
    }
}
