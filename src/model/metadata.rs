pub use serde::Deserialize;
use crate::util::mydatetime::MyDateTime;

use super::{Error, IndexMetadata};

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Metadata {
    pub title: String,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub created: Option<MyDateTime>,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Metadata {
    pub fn from_yaml<S: AsRef<str>>(yaml: S) -> Result<Self, Error> {
        let deserializer = serde_yaml::Deserializer::from_str(yaml.as_ref());
        Ok(Self::deserialize(deserializer)?)
    }
}

impl From<IndexMetadata> for Metadata {
    fn from(value: IndexMetadata) -> Self {
        Self {
            title: value.title,
            author: value.author,
            summary: value.summary,
            created: None,
            highlight: value.highlight,
            tags: value.tags,
        }
    }
}

impl PartialOrd for Metadata {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.created.partial_cmp(&other.created)
    }
}
