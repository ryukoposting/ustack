pub use serde::Deserialize;
use serde::{de::Error, Deserializer};
use url::Url;

#[derive(Debug, Deserialize, Clone)]
pub struct IndexMetadata {
    pub title: String,
    pub short_title: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(deserialize_with = "deserialize_url")]
    pub url: Url,
    #[serde(default)]
    pub twitter: bool,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default, deserialize_with = "deserialize_opt_url")]
    pub coffee: Option<Url>,
}

impl IndexMetadata {
    pub fn from_yaml<S: AsRef<str>>(yaml: S) -> Result<Self, super::Error> {
        let deserializer = serde_yaml::Deserializer::from_str(yaml.as_ref());
        Ok(Self::deserialize(deserializer)?)
    }

    pub fn twitter_link(&self, post_id: &str) -> Result<Option<Url>, Box<dyn std::error::Error>> {
        let mut url = self.url.clone();
        if self.twitter {
            url.set_path(&format!("p/{post_id}"));
            Ok(Some(url))
        } else {
            Ok(None)
        }
    }
}

fn default_lang() -> String {
    "en_US".to_string()
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let url_str = String::deserialize(deserializer)?;
    let url = Url::parse(&url_str)
        .map_err(|err| D::Error::custom(format!("{err}")))?;
    if url.cannot_be_a_base() {
        Err(D::Error::custom("Index URL must be a base URL"))
    } else {
        Ok(url)
    }
}

fn deserialize_opt_url<'de, D>(deserializer: D) -> Result<Option<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let url_str = String::deserialize(deserializer)?;
    let url = Url::parse(&url_str)
        .map_err(|err| D::Error::custom(format!("{err}")))?;
    if url.cannot_be_a_base() {
        Err(D::Error::custom("Index URL must be a base URL"))
    } else {
        Ok(Some(url))
    }
}

impl Default for IndexMetadata {
    fn default() -> Self {
        Self {
            // TODO: replace impl of Default with a non-hacky initializer for IndexMetadata
            url: Url::parse("https://unspecified.com").unwrap(),
            title: Default::default(),
            author: Default::default(),
            summary: Default::default(),
            highlight: Default::default(),
            tags: Default::default(),
            twitter: Default::default(),
            lang: Default::default(),
            coffee: Default::default(),
            short_title: Default::default(),
        }
    }
}
