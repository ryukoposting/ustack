use std::{error::Error, io, path::{PathBuf, Path}, fs};
use clap::{Parser, Subcommand};
use log::{LevelFilter, debug, warn};

#[derive(Debug, Parser)]
pub struct Generate {
    /// Adjusts the verbosity of the logger.
    #[arg(short, long, default_value = "warn")]
    pub log_level: LevelFilter,
    #[command(subcommand)]
    what: What
}

#[derive(Debug, Subcommand)]
enum What {
    /// Generate a new blog post.
    Post {
        /// ID of the new blog post.
        /// 
        /// The ID should consist solely of the characters a-z, A-Z, 0-9, and hyphen.
        id: String
    }
}

const GENERATED_POST: &str = include_str!("res/generate_post.md");

impl Generate {
    pub fn run(self) -> Result<(), Box<dyn Error>> {
        match self.what {
            What::Post { id } => Self::generate_post(id),
        }
    }

    fn generate_post(id: String) -> Result<(), Box<dyn Error>> {
        if !id.chars().all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-')) {
            return Err(format!("Invalid post id '{id}'").into());
        }

        let mut path = Path::new("posts").join(&id);
        path.set_extension("md");

        if path.exists() {
            return Err(format!("A post with this ID already exists!").into());
        }

        Ok(fs::write(path, GENERATED_POST)?)
    }
}
