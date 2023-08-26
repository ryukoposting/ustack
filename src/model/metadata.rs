pub use serde::Deserialize;
use super::Error;

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub title: String,
    pub author: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub canonical: Option<String>,
}

impl Metadata {
    pub fn from_yaml<S: AsRef<str>>(yaml: S) -> Result<Self, Error> {
        let deserializer = serde_yaml::Deserializer::from_str(yaml.as_ref());
        Ok(Self::deserialize(deserializer)?)
    }
}
