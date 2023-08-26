mod metadata;

pub use metadata::*;

use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    Yaml(serde_yaml::Error),
    Io(std::io::Error),
}

impl From<serde_yaml::Error> for Error {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Yaml(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        use std::io::{self, ErrorKind};
        match value {
            Error::Yaml(yaml) => io::Error::new(
                ErrorKind::InvalidData,
                format!("YAML parse error: {yaml}")
            ),
            Error::Io(io) => io,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Yaml(err) => err.fmt(f),
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {}
