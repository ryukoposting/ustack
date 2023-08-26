//! The blog "database" holds the logic for post caching. It serves as the interface

use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    path::PathBuf,
    time::{Duration, SystemTime}, cmp::max, cell::RefCell, error::Error,
};

use chrono::{DateTime, Local};
use comrak::{nodes::{NodeValue::FrontMatter, Ast}, Arena, ComrakOptions, arena_tree::Node};
use log::{debug, error, info, warn};
use url::Url;
use crate::model::{Metadata, IndexMetadata};
use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
};

pub struct PostDb {
    posts: HashMap<String, PostEntry>,
    posts_dir: PathBuf,
    ttl: Duration,
    index_updated: SystemTime,
    index_metadata: IndexMetadata,
}

pub struct PostEntry {
    updated: SystemTime,
    last_modified: SystemTime,
    metadata: Metadata,
    body: String,
}

pub struct Post<'a> {
    id: &'a str,
    entry: &'a PostEntry,
}

#[derive(Debug, PartialEq)]
pub struct PostMeta {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct PostContent {
    pub id: String,
    pub body: String,
    pub last_modified: SystemTime,
    pub metadata: Metadata,
}

impl PostDb {
    pub fn new(posts_dir: PathBuf, ttl_seconds: u32) -> Result<Self, io::Error> {
        Ok(Self {
            posts: HashMap::default(),
            posts_dir: dunce::canonicalize(posts_dir)?,
            ttl: Duration::from_secs(ttl_seconds as u64),
            index_updated: SystemTime::UNIX_EPOCH,
            index_metadata: IndexMetadata::default(),
        })
    }

    pub fn get<'a>(&'a self, id: &'a str) -> Option<Post<'a>> {
        self.posts.get(id).map(|entry| Post { id, entry })
    }

    pub fn all_posts<'a>(&'a self) -> impl Iterator<Item = Post<'a>> {
        self.posts
            .iter()
            .filter(|(id, _)| !id.starts_with("/"))
            .map(|(id, entry)| Post { id, entry })
    }

    /// The last time any file in the db was modified
    pub fn index_updated(&self) -> DateTime<Local> {
        self.index_updated.into()
    }

    /// Generates a twitter sharing link
    pub fn twitter_link(&self, id: &str) -> Result<Option<Url>, Box<dyn Error>> {
        self.index_metadata.twitter_link(id)
    }

    /// Blog title
    pub fn site_title(&self) -> &str {
        &self.index_metadata.title
    }

    /// Blog URL
    pub fn site_url(&self) -> Result<Url, url::ParseError> {
        Url::parse(&self.index_metadata.url)
    }

    pub fn lang(&self) -> &str {
        &self.index_metadata.lang
    }

    pub async fn refresh_index<'a>(&'a mut self, allow_search_all: bool) -> Result<Post<'a>, io::Error> {
        if self.index_updated + self.ttl <= SystemTime::now() && allow_search_all {
            let mut posts_dir_iter = fs::read_dir(&self.posts_dir).await?;
            while let Some(ent) = posts_dir_iter.next_entry().await? {
                let path = PathBuf::from(ent.file_name());
                if !path.extension().map_or(false, |ext| ext == "md") {
                    continue;
                }

                if let Some(id) = path.with_extension("").file_name().and_then(|s| s.to_str()) {
                    debug!("refreshing");
                    if !self.posts.contains_key(id) {
                        self.refresh(id).await?;
                    }
                } else {
                    debug!("not valid");
                }
            }
            self.index_updated = SystemTime::now();
        }

        let post_file = dunce::canonicalize(self.posts_dir.join("../index.md"))?;
        self.refresh_inner("/index", post_file).await
    }

    pub async fn refresh<'a>(&'a mut self, id: &'a str) -> Result<Post<'a>, io::Error> {
        let post_file = match dunce::canonicalize(self.posts_dir.join(id).with_extension("md")) {
            Ok(ok) => {
                if ok.starts_with(&self.posts_dir) {
                    ok
                } else {
                    warn!("Suspicious refresh request for id={id:?} did not start with canonical posts_dir");
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("Invalid post id {id:?}"),
                    ));
                }
            }
            Err(err) => {
                info!("Refresh request for id={id:?} caused error: {err}");
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid post id {id:?}"),
                ));
            }
        };

        self.refresh_inner(id, post_file).await
    }

    async fn refresh_inner<'a>(
        &'a mut self,
        id: &'a str,
        post_file: PathBuf
    ) -> Result<Post<'a>, io::Error> {
        let updated = self.posts.get(id).map(|ent| ent.updated);

        if updated.map_or(false, |updated| updated + self.ttl >= SystemTime::now()) {
            // file is not due for another check yet
            return Ok(self.get(id).unwrap());
        }

        let file = File::open(&post_file).await.map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                debug!("No such post with id {id}, trying to delete it from cache");
                self.posts.remove(id);
                err
            } else {
                error!("{err} (opening {post_file:?})");
                err
            }
        })?;

        let file_modified_time = file.metadata().await?.modified()?;

        if updated.map_or(false, |updated| updated >= file_modified_time) {
            // file has not been changed since last check
            self.posts.get_mut(id).unwrap().updated = SystemTime::now();
            return Ok(self.get(id).unwrap());
        }

        if id == "/index" {
            self.parse_index(file).await?;
        } else {
            self.parse_page(file, id).await?;
        }

        Ok(self.get(id).unwrap())
    }

    async fn parse_index(&mut self, file: File) -> Result<(), io::Error> {
        let (entry, meta) = PostEntry::parse_index(file).await?;

        self.index_updated = max(entry.last_modified, self.index_updated);
        self.index_metadata = meta;
        self.posts.insert("/index".to_string(), entry);

        info!("Refreshed /index");

        Ok(())
    }

    async fn parse_page(&mut self, file: File, id: &str) -> Result<(), io::Error> {
        let entry = PostEntry::parse(file).await?;

        self.posts.insert(id.to_string(), entry);

        info!("Refreshed {id}");

        Ok(())
    }
}

struct Parser<'a> {
    arena: Arena<Node<'a, RefCell<Ast>>>,
    buffer: String,
    options: ComrakOptions,
}

impl<'a> Parser<'a> {
    fn new(buffer: String) -> Self {
        let mut options = ComrakOptions::default();
        options.extension.front_matter_delimiter = Some("---".into());
        options.extension.strikethrough = true;
        options.extension.header_ids = Some("".to_string());
        options.extension.table = true;
        options.extension.tasklist = true;
        options.render.unsafe_ = true;
        options.parse.smart = true;
        options.parse.relaxed_tasklist_matching = true;

        Self {
            arena: Arena::new(),
            buffer,
            options
        }
    }

    fn parse(&'a self) -> Result<&'a Node<'_, RefCell<Ast>>, io::Error> {
        Ok(comrak::parse_document(&self.arena, &self.buffer, &self.options))
    }

    fn generate_html(&self, root: &'a Node<'a, RefCell<Ast>>) -> Result<Vec<u8>, io::Error> {
        let mut html = vec![];
        comrak::format_html(root, &self.options, &mut html)?;
        Ok(html)
    }

    fn get_metadata(&self, root: &'a Node<'a, RefCell<Ast>>) -> Result<Metadata, io::Error> {
        let front_matter = root
            .clone()
            .children()
            .filter_map(|child| {
                let data = child.data.borrow();
                match &data.value {
                    FrontMatter(fm) => Some(fm.clone()),
                    _ => None,
                }
            })
            .nth(0);

        if let Some(fm) = front_matter {
            let fm = fm
                .trim()
                .strip_prefix("---")
                .unwrap()
                .strip_suffix("---")
                .unwrap();

            Ok(Metadata::from_yaml(fm)?)
        } else {
            Err(io::Error::new(ErrorKind::InvalidData, "Missing a YAML preamble"))
        }
    }

    fn get_index_metadata(&self, root: &'a Node<'a, RefCell<Ast>>) -> Result<IndexMetadata, io::Error> {
        let front_matter = root
            .clone()
            .children()
            .filter_map(|child| {
                let data = child.data.borrow();
                match &data.value {
                    FrontMatter(fm) => Some(fm.clone()),
                    _ => None,
                }
            })
            .nth(0);

        if let Some(fm) = front_matter {
            let fm = fm
                .trim()
                .strip_prefix("---")
                .unwrap()
                .strip_suffix("---")
                .unwrap();

            Ok(IndexMetadata::from_yaml(fm)?)
        } else {
            Err(io::Error::new(ErrorKind::InvalidData, "Missing a YAML preamble"))
        }
    }
}

impl PostEntry {
    pub async fn parse_index(mut file: File) -> Result<(Self, IndexMetadata), io::Error> {
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).await?;

        let last_modified = file.metadata().await?.modified()?;
    
        let parser = Parser::new(buffer);
        let root = parser.parse()?;
        let html = parser.generate_html(root)?;
        let metadata = parser.get_index_metadata(root)?;

        let entry = Self {
            updated: SystemTime::now(),
            last_modified,
            metadata: metadata.clone().into(),
            body: String::from_utf8_lossy(&html).to_string(),
        };

        Ok((entry, metadata))
    }

    pub async fn parse(mut file: File) -> Result<Self, io::Error> {
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).await?;

        let last_modified = file.metadata().await?.modified()?;
    
        let parser = Parser::new(buffer);
        let root = parser.parse()?;
        let html = parser.generate_html(root)?;
        let metadata = parser.get_metadata(root)?;

        let entry = Self {
            updated: SystemTime::now(),
            last_modified,
            metadata: metadata.into(),
            body: String::from_utf8_lossy(&html).to_string(),
        };

        Ok(entry)
    }
}

impl<'a> Post<'a> {
    pub fn updated(&self) -> SystemTime {
        self.entry.updated
    }

    pub fn id(&self) -> &'a str {
        self.id
    }

    pub fn body(&self) -> &'a str {
        &self.entry.body
    }

    pub fn metadata(&self) -> &'a Metadata {
        &self.entry.metadata
    }

    pub fn to_post_meta(&self) -> PostMeta {
        PostMeta {
            id: self.id().to_string(),
            title: self.metadata().title.to_string(),
            summary: self.metadata().summary.as_ref().map(|s| s.to_string()),
        }
    }

    pub fn to_post_content(&self) -> PostContent {
        PostContent {
            id: self.id().to_string(),
            body: self.body().to_string(),
            last_modified: self.entry.last_modified,
            metadata: self.metadata().clone(),
        }
    }
}

impl PostContent {
    pub fn last_modified(&self) -> DateTime<Local> {
        DateTime::from(self.last_modified)
    }
}
