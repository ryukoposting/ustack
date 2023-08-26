pub use serde::Deserialize;
use url::Url;
use super::Error;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct IndexMetadata {
    pub title: String,
    pub author: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    pub url: String,
    #[serde(default)]
    pub twitter: bool,
}

impl IndexMetadata {
    pub fn from_yaml<S: AsRef<str>>(yaml: S) -> Result<Self, Error> {
        let deserializer = serde_yaml::Deserializer::from_str(yaml.as_ref());
        Ok(Self::deserialize(deserializer)?)
    }

    pub fn twitter_link(&self, post_id: &str) -> Result<Option<Url>, Box<dyn std::error::Error>> {
        let mut url = Url::parse(&self.url)?;
        if self.twitter {
            url.set_path(&format!("p/{post_id}"));
            Ok(Some(url))
        } else {
            Ok(None)
        }
    }
}
