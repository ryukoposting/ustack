//! The blog "database" holds the logic for post caching. It serves as the interface

use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    path::PathBuf,
    time::{Duration, SystemTime}, cmp::max,
};

use chrono::{DateTime, Local};
use comrak::{nodes::NodeValue::FrontMatter, Arena, ComrakOptions};
use log::{debug, error, info, warn};
use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
};

use crate::model::{Blog, Metadata};

pub struct PostDb {
    posts: HashMap<String, PostEntry>,
    posts_dir: PathBuf,
    ttl: Duration,
    index_updated: SystemTime,
}

pub struct PostEntry {
    updated: SystemTime,
    last_modified: SystemTime,
    body: String,
    title: String,
    author: Option<String>,
    summary: String,
    highlight: bool,
}

pub struct Post<'a> {
    id: &'a str,
    entry: &'a PostEntry,
}

#[derive(Debug, PartialEq)]
pub struct PostMeta {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, PartialEq)]
pub struct PostContent {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub body: String,
    pub timestamp: DateTime<Local>,
    pub highlight: bool,
}

impl PostDb {
    pub fn new(posts_dir: PathBuf, ttl_seconds: u32) -> Result<Self, io::Error> {
        Ok(Self {
            posts: HashMap::default(),
            posts_dir: dunce::canonicalize(posts_dir)?,
            ttl: Duration::from_secs(ttl_seconds as u64),
            index_updated: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn get<'a>(&'a self, id: &'a str) -> Option<Post<'a>> {
        self.posts.get(id).map(|entry| Post { id, entry })
    }

    pub fn all_posts<'a>(&'a self) -> impl Iterator<Item = Post<'a>> {
        self.posts
            .iter()
            .filter(|(id, _)| id.as_str() != "")
            .map(|(id, entry)| Post { id, entry })
    }

    /// The last time any file in the db was modified
    pub fn index_updated(&self) -> DateTime<Local> {
        self.index_updated.into()
    }

    pub fn site_title(&self) -> Option<&str> {
        self.get("").map(|index| index.title())
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
        self.refresh_inner("", post_file, true).await
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

        self.refresh_inner(id, post_file, false).await
    }

    async fn refresh_inner<'a>(
        &'a mut self,
        id: &'a str,
        post_file: PathBuf,
        is_index: bool,
    ) -> Result<Post<'a>, io::Error> {
        // let res_dir = post_file.with_extension(""); // TODO: index posts dir for resource file changes

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

        let mut entry = PostEntry::new();

        entry.parse(file, is_index).await?;

        self.posts.insert(id.to_string(), entry);

        if is_index {
            info!("Refreshed index");
        } else {
            info!("Refreshed post {id}");
        }

        self.posts.get_mut(id).unwrap().last_modified = file_modified_time;

        self.index_updated = max(file_modified_time, self.index_updated);

        Ok(self.get(id).unwrap())
    }
}

impl PostEntry {
    fn new() -> Self {
        PostEntry {
            updated: SystemTime::now(),
            last_modified: SystemTime::UNIX_EPOCH,
            body: String::default(),
            title: String::default(),
            summary: String::default(),
            author: None,
            highlight: false,
        }
    }

    async fn parse(&mut self, mut file: File, is_index: bool) -> Result<(), io::Error> {
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).await?;

        let arena = Arena::new();

        let mut options = ComrakOptions::default();
        options.extension.front_matter_delimiter = Some("---".into());
        options.extension.strikethrough = true;
        options.extension.header_ids = Some("p-".to_string());
        options.extension.table = true;
        options.extension.tasklist = true;
        options.parse.smart = true;
        options.parse.relaxed_tasklist_matching = true;

        let root = comrak::parse_document(&arena, &buffer, &options);

        let mut html = vec![];
        comrak::format_html(root, &options, &mut html)?;

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

            if !is_index {
                let fm = Metadata::from_yaml(fm)?;
                self.title = fm.title;
                self.author = fm.author;
                self.summary = fm.summary;
                self.highlight = fm.highlight;
            } else {
                let fm = Blog::from_yaml(fm)?;
                self.title = fm.title;
                self.highlight = fm.highlight;
            }
        }

        self.body = String::from_utf8_lossy(&html).to_string();

        Ok(())
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

    pub fn title(&self) -> &'a str {
        &self.entry.title
    }

    pub fn author(&self) -> Option<&'a str> {
        self.entry.author.as_deref()
    }

    pub fn summary(&self) -> &'a str {
        &self.entry.summary
    }

    pub fn to_post_meta(&self) -> PostMeta {
        PostMeta {
            id: self.id().to_string(),
            title: self.title().to_string(),
            summary: self.summary().to_string(),
        }
    }

    pub fn to_post_content(&self) -> PostContent {
        PostContent {
            id: self.id().to_string(),
            title: self.title().to_string(),
            body: self.body().to_string(),
            author: self.author().map(|a| a.to_string()),
            timestamp: self.entry.last_modified.into(),
            highlight: self.entry.highlight,
        }
    }
}
