use std::{error::Error, fs, path::Path, io::Write};

use clap::Parser;
use log::{LevelFilter, debug, warn};

#[derive(Debug, Parser)]
pub struct Init {
    /// Adjusts the verbosity of the logger.
    #[arg(long, default_value = "warn")]
    pub log_level: LevelFilter,
}

const DEFAULT_STYLES: &str = include_str!("res/default_styles.css");
const DEFAULT_CONFIG: &str = include_str!("res/default_index.md");
const SAMPLE_POST: &str = include_str!("res/sample_post.md");

impl Init {
    pub fn run(self) -> Result<(), Box<dyn Error>> {
        debug!("Creating directories");
        fs::create_dir_all("posts")?;
        fs::create_dir_all("public")?;

        if Path::new("public/styles.css").exists() {
            warn!("Not creating styles.css because it already exists");
        } else {
            debug!("Creating default styles.css");
            let mut file = fs::File::create("public/styles.css")?;
            file.write_all(DEFAULT_STYLES.as_bytes())?;
        }

        if Path::new("index.md").exists() {
            warn!("Not creating index.md because it already exists");
        } else {
            debug!("Creating default index.md");
            let mut file = fs::File::create("index.md")?;
            file.write_all(DEFAULT_CONFIG.as_bytes())?;
        }

        if Path::new("posts/sample-post.md").exists() {
            warn!("Not creating posts/sample-post.md because it already exists");
        } else {
            debug!("Creating posts/sample-post.md");
            let mut file = fs::File::create("posts/sample-post.md")?;
            file.write_all(SAMPLE_POST.as_bytes())?;
        }

        Ok(())
    }
}
