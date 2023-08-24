mod cli;
mod init;
mod serve;
mod view;
mod util;
mod model;

use std::error::Error;

use clap::Parser;
use cli::Action;
use dioxus::prelude::*;

use crate::cli::Cli;

pub const DEFAULT_STYLES: &str = include_str!("default_styles.css");
pub const DEFAULT_CONFIG: &str = include_str!("default_index.md");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    simple_logger::SimpleLogger::new()
        .with_level(args.action.log_level())
        .init()
        .expect("Initializing logger");

    match args.action {
        Action::Serve(serve) => serve.run().await?,
        Action::Init(init) => init.run().await?,
    }

    let mut vdom = VirtualDom::new(app);
    let _ = vdom.rebuild();

    // let text = dioxus_ssr::render(&vdom);
    // println!("{text}");

    Ok(())
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        main {
            h1 { "Hello world!" }
            p { "paragraph text!" }
        }
    })
}
