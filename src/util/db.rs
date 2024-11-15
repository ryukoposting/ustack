//! The blog "database" holds the logic for post caching.

use std::{
    cell::RefCell,
    cmp::{max, Ordering},
    collections::HashMap,
    error::Error,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use crate::{model::{IndexMetadata, Metadata}, util};
use super::mydatetime::MyDateTime;
use chrono::{DateTime, FixedOffset, Local};
use comrak::{
    arena_tree::Node,
    nodes::{Ast, NodeValue::FrontMatter},
    Arena, ComrakOptions,
};
use itertools::Itertools;
use log::{debug, error, info, warn};
use rand::{seq::IteratorRandom, thread_rng};
use rss::{ChannelBuilder, extension::atom::{AtomExtensionBuilder, Link}, ImageBuilder};
use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
};
use url::Url;

pub struct PostDb {
    posts: HashMap<String, PostEntry>,
    posts_dir: PathBuf,
    ttl: Duration,
    index_updated: SystemTime,
    index_metadata: IndexMetadata,
    rss_base: ChannelBuilder
}

#[derive(PartialEq, PartialOrd)]
pub struct PostEntry {
    /// The last time the database updated this PostEntry
    updated: SystemTime,
    /// The last time the blog file was modified
    last_modified: SystemTime,
    metadata: Metadata,
    body: String,
}

pub struct Post<'a> {
    id: &'a str,
    entry: &'a PostEntry,
    db: &'a PostDb
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
            rss_base: ChannelBuilder::default()
        })
    }

    pub fn get<'a>(&'a self, id: &'a str) -> Option<Post<'a>> {
        self.posts.get(id).map(|entry| Post { id, entry, db: self })
    }

    pub fn get_random_id<'a>(&'a self) -> Option<&'a str> {
        let mut rng = thread_rng();
        let choices = self.posts.keys()
            .filter(|id| !id.starts_with('/'))
            .choose(&mut rng);
        choices.map(|id| id.as_str())
    }

    pub fn all_posts<'a>(&'a self) -> impl Iterator<Item = Post<'a>> {
        self.posts
            .iter()
            .filter(|(id, _)| !id.starts_with("/"))
            .map(|(id, entry)| Post { id, entry, db: self })
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

    /// Blog title
    pub fn site_title_short(&self) -> &str {
        let site_title = self.site_title();
        self.index_metadata.short_title
            .as_deref()
            .unwrap_or(site_title)
    }

    /// Blog URL
    pub fn site_url(&self) -> &Url {
        &self.index_metadata.url
    }

    /// Coffee URL
    pub fn coffee_url(&self) -> Option<&Url> {
        self.index_metadata.coffee.as_ref()
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Post URL
    pub fn post_url(&self, post: &Post<'_>) -> Url {
        let mut result = self.site_url().clone();
        result.path_segments_mut()
            .expect("site_url shall be a base")
            .extend(&["p", post.id()]);
        result
    }

    pub fn site_summary(&self) -> Option<&str> {
        self.index_metadata.summary.as_deref()
    }

    pub fn lang(&self) -> &str {
        &self.index_metadata.lang
    }
    
    pub async fn refresh_index<'a>(
        &'a mut self,
        allow_search_all: bool,
    ) -> Result<Post<'a>, io::Error> {
        if allow_search_all && self.index_updated + self.ttl <= SystemTime::now() {
            let mut posts_dir_iter = fs::read_dir(&self.posts_dir).await?;
            while let Some(ent) = posts_dir_iter.next_entry().await? {
                let path = PathBuf::from(ent.file_name());

                let is_markdown = path.extension().map_or(false, |ext| ext == "md");
                let is_dotted = path
                    .file_name()
                    .map_or(false, |name| name.to_string_lossy().starts_with('.'));

                if is_dotted || !is_markdown {
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

    pub fn get_rss(&self, since: Option<&DateTime<FixedOffset>>, include_content: bool, max: usize) -> ChannelBuilder
    {
        let mut builder = self.rss_base.clone();

        let items = self.all_posts()
            .filter(|p| p.metadata().created.as_deref() >= since)
            .sorted_by(|a, b| b.cmp_published(a))
            .take(max)
            .map(|p| p.to_rss_item(include_content))
            .collect_vec();

        builder.items(items);

        builder
    }

    fn validate_post_path(&self, id: &str, path: &Path) -> Result<(), io::Error> {
        fn invalid_path(id: &str) -> io::Error {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("Invalid post id {id:?}")
            )
        }

        let valid_filename =
            path.extension().map_or(false, |ext| ext == "md") &&
            path.file_name().map_or(false, |name| {
                name.to_str().map_or(false, |name| name.starts_with(|c| matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9')))
            }
        );

        let valid_parent_path = path.starts_with(&self.posts_dir);

        if !valid_parent_path {
            warn!("Suspicious post id={id:?} did not start with canonical posts_dir");
            Err(invalid_path(id))
        } else if !valid_filename {
            Err(invalid_path(id))
        } else {
            Ok(())
        }
    }

    fn get_unvalidated_post_path(&self, id: &str) -> Result<PathBuf, io::Error> {
        dunce::canonicalize(self.posts_dir.join(id).with_extension("md"))
    }

    /// Refresh db entry for a particular post
    pub async fn refresh<'a>(&'a mut self, id: &'a str) -> Result<Post<'a>, io::Error> {
        let post_file = match self.get_unvalidated_post_path(id) {
            Ok(path) => {
                self.validate_post_path(id, &path)?;
                path
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    debug!("No such post with id {id}, trying to delete it from cache");
                    self.posts.remove(id);
                    return Err(err);
                } else {
                    info!("Refresh request for id={id:?} caused error: {err}");
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("Invalid post id {id:?}"),
                    ));
                }
            }
        };

        self.refresh_inner(id, post_file).await
    }

    async fn refresh_inner<'a>(
        &'a mut self,
        id: &'a str,
        post_file: PathBuf,
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
        self.rss_base = self.make_rss_base();

        info!("Refreshed /index and RSS");

        Ok(())
    }

    async fn parse_page(&mut self, file: File, id: &str) -> Result<(), io::Error> {
        let entry = PostEntry::parse(file).await?;

        self.posts.insert(id.to_string(), entry);

        info!("Refreshed {id}");

        Ok(())
    }

    fn make_rss_base(&self) -> ChannelBuilder
    {
        use quick_xml::escape::partial_escape;

        let mut channel = rss::ChannelBuilder::default();
        channel.title(partial_escape(self.site_title()));
        channel.link(partial_escape(self.site_url().as_str()));
        channel.language(Some(partial_escape(self.lang()).to_string()));
        channel.last_build_date(Some(MyDateTime::from(self.index_updated).to_string_rss()));

        let ttl_as_minutes = (self.ttl.as_secs() + 59) / 60;
        let ttl_as_minutes = std::cmp::max(ttl_as_minutes, 5);
        channel.ttl(Some(ttl_as_minutes.to_string()));

        channel.pub_date(Some(MyDateTime::now().to_string_rss()));

        channel.image(Some(ImageBuilder::default()
            .url({
                let mut url = self.site_url().clone();
                url.path_segments_mut().unwrap()
                    .extend(&["public", "favicon.png"]);
                url.to_string()
            })
            .title(self.site_title().to_string())
            .link(self.site_url().to_string())
            .build()
        ));

        let atom = AtomExtensionBuilder::default()
            .links(vec![
                {
                    let mut rss_path = self.site_url().clone();
                    rss_path.path_segments_mut().unwrap().extend(&["rss"]);
                    let mut link = Link::default();
                    link.set_href(rss_path);
                    link.set_rel("self");
                    link.set_mime_type(Some("application/rss+xml".to_string()));
                    link
                }
            ])
            .build();
        channel.atom_ext(Some(atom));

        if let Some(summary) = self.site_summary() {
            channel.description(summary.to_string());
        }

        channel
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
            options,
        }
    }

    fn parse(&'a self) -> Result<&'a Node<'_, RefCell<Ast>>, io::Error> {
        Ok(comrak::parse_document(
            &self.arena,
            &self.buffer,
            &self.options,
        ))
    }

    fn generate_html(&self, root: &'a Node<'a, RefCell<Ast>>) -> Result<Vec<u8>, io::Error> {
        let mut html = vec![];
        comrak::format_html(root, &self.options, &mut html)?;
        Ok(html)
    }

    fn get_metadata(&self, root: &'a Node<'a, RefCell<Ast>>) -> Result<Metadata, io::Error> {
        let front_matter = root
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
            Err(io::Error::new(
                ErrorKind::InvalidData,
                "Missing a YAML preamble",
            ))
        }
    }

    fn get_index_metadata(
        &self,
        root: &'a Node<'a, RefCell<Ast>>,
    ) -> Result<IndexMetadata, io::Error> {
        let front_matter = root
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
            Err(io::Error::new(
                ErrorKind::InvalidData,
                "Missing a YAML preamble",
            ))
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
    pub fn cmp_published(&self, other: &Post) -> Ordering {
        match (&self.entry.metadata.created, &other.entry.metadata.created) {
            (None, None) => self.entry.last_modified.cmp(&other.entry.last_modified),
            (None, Some(b)) => self.entry.last_modified.cmp(&b.system_time()),
            (Some(a), None) => a.system_time().cmp(&other.entry.last_modified),
            (Some(a), Some(b)) => a.system_time().cmp(&b.system_time()),
        }
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

    pub fn to_rss_item(&self, include_content: bool) -> rss::Item {
        use quick_xml::escape::partial_escape;

        let url = self.db.post_url(self).to_string();
        let guid = rss::GuidBuilder::default()
            .value(url.clone())
            .permalink(true)
            .build();
        let pub_date: Option<String> = self.metadata().created.as_ref()
            .map(|t| t.to_string_rss());

        let mut item = rss::ItemBuilder::default();
        item.title(Some(partial_escape(&self.metadata().title).to_string()));
        item.pub_date(pub_date);
        item.link(Some(url));
        item.guid(Some(guid));
        item.description(
            self.metadata().summary.as_ref()
                .map(|s| partial_escape(s).to_string()));

        if include_content {
            item.content(Some(format!("{}{}",
                util::render_base_part(self.db.site_url()),
                self.body())));
        }

        item.build()
    }
}

impl PostContent {
    pub fn published(&self) -> DateTime<FixedOffset> {
        if let Some(created) = &self.metadata.created {
            created.fixed_offset()
        } else {
            self.last_modified().fixed_offset()
        }
    }

    pub fn last_modified(&self) -> DateTime<Local> {
        DateTime::from(self.last_modified)
    }
}
