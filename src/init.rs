use std::error::Error;

use clap::Parser;
use log::{LevelFilter, debug, warn};
use tokio::{fs, io::AsyncWriteExt};

#[derive(Debug, Parser)]
pub struct Init {
    /// Adjusts the verbosity of the logger.
    #[arg(long, default_value = "warn")]
    pub log_level: LevelFilter,
}

impl Init {
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        debug!("Creating posts directory");
        fs::create_dir_all("posts").await?;

        if fs::try_exists("styles.css").await? {
            warn!("Not creating styles.css because it already exists");
        } else {
            debug!("Creating default styles.css");
            let mut file = fs::File::create("styles.css").await?;
            file.write_all(crate::DEFAULT_STYLES.as_bytes()).await?;
        }

        if fs::try_exists("index.md").await? {
            warn!("Not creating index.md because it already exists");
        } else {
            debug!("Creating default styles.css");
            let mut file = fs::File::create("index.md").await?;
            file.write_all(crate::DEFAULT_CONFIG.as_bytes()).await?;
        }

        Ok(())
    }
}
