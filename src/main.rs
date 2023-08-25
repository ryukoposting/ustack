mod cli;
mod generate;
mod init;
mod serve;
mod view;
mod util;
mod model;

use std::error::Error;

use clap::Parser;
use cli::Action;

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    simple_logger::SimpleLogger::new()
        .with_level(args.action.log_level())
        .init()
        .expect("Initializing logger");

    match args.action {
        Action::Serve(serve) => serve.run().await?,
        Action::Init(init) => init.run()?,
        Action::Generate(generate) => generate.run()?,
    }

    Ok(())
}
