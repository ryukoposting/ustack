//! `serve` command handler.

use chrono::{DateTime, Local};
use clap::{Parser, ValueEnum};
use dioxus::prelude::*;
use hyper::{
    header::{CACHE_CONTROL, CONTENT_TYPE, LAST_MODIFIED, VARY},
    server::conn::AddrStream,
    service::service_fn,
    Body, Method, Request, Response, StatusCode,
};
use itertools::Itertools;
use log::{debug, error, info, LevelFilter};
use std::{
    convert::Infallible, env, error::Error, io::ErrorKind, net::SocketAddr, num::NonZeroUsize,
    path::PathBuf, sync::Arc,
};
use tokio::{fs::File, io::AsyncReadExt, sync::RwLock};
use url::Url;

use crate::{
    util::{
        self,
        header_ext::HeaderExt,
        db::{PostContent, PostDb},
    },
    view::{self, IndexProps, NotFoundProps, PostProps},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum RssContent {
    Never,
    SupportsDeltas,
    Always,
}

#[derive(Debug, Parser)]
pub struct Serve {
    /// Root directory of the mdblog project
    #[arg(short, long)]
    directory: Option<PathBuf>,

    /// Address and port the server will use.
    #[arg(short, long, default_value = "127.0.0.1:4198")]
    address: SocketAddr,

    /// Post cache time-to-live, in seconds. Lower values result in more frequent updates to served content.
    ///
    /// Values below the default are not recommended for production servers.
    #[arg(short = 'c', long, default_value = "300")]
    cache_ttl: u32,

    /// Maximum number of posts shown on each page of the index.
    #[arg(long, default_value = "10")]
    index_page_len: NonZeroUsize,

    /// Adjusts the verbosity of the logger.
    #[arg(long, default_value = "warn")]
    pub log_level: LevelFilter,

    /// When to include post content in RSS feed data
    #[arg(long, default_value = "supports-deltas")]
    rss_content: RssContent
}

struct Server {
    db: PostDb,
    // address: SocketAddr,
    index_page_len: usize,
    rss_content: RssContent,
    public_dir: PathBuf,
}

const ROBOTS_TXT: &str = include_str!("res/robots.txt");

impl Serve {
    pub fn directory(&self) -> Result<PathBuf, std::io::Error> {
        self.directory
            .as_ref()
            .map_or_else(|| env::current_dir(), |path| dunce::canonicalize(path))
    }

    fn into_server(self) -> Result<Server, Box<dyn Error>> {
        let dir = self.directory()?;
        let posts_dir = dir.join("posts");
        let public_dir = dir.join("public");

        let db = PostDb::new(posts_dir, self.cache_ttl)?;

        let server = Server {
            db,
            index_page_len: self.index_page_len.into(),
            public_dir,
            rss_content: self.rss_content
        };
        Ok(server)
    }

    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let address = self.address.clone();
        let server = self.into_server()?;
        let server = Arc::from(RwLock::new(server));

        let make_service = hyper::service::make_service_fn(|conn: &AddrStream| {
            let address = conn.remote_addr();

            let server = server.clone();

            let service =
                service_fn(move |request| Server::handle(server.clone(), address, request));

            async move { Ok::<_, Infallible>(service) }
        });

        info!("Listening on http://{}", address);

        hyper::Server::bind(&address).serve(make_service).await?;

        Ok(())
    }
}

impl Server {
    async fn handle(
        server: Arc<RwLock<Server>>,
        client_addr: SocketAddr,
        req: Request<Body>,
    ) -> Result<Response<Body>, hyper::http::Error> {
        debug!("{client_addr} {} {:?}", req.method(), req.uri());

        let req_uri = req.uri().path();

        let result = if req.method() == Method::GET && (req_uri == "/" || req_uri == "/rss") {
            let index = {
                let mut server = server.write().await;
                server
                    .db
                    .refresh_index(true)
                    .await
                    .map(|post| post.to_post_content())
            };

            match index {
                Ok(index) => if req_uri == "/rss" {
                    server.read().await.rss(req).await
                } else {
                    server.read().await.index(req, index).await
                },
                Err(err) => Err(err.into()),
            }
        } else if req.method() == Method::GET && req_uri.starts_with("/p/") {
            let post = {
                let id = req_uri.split('/').nth(2).unwrap_or("");
                let mut server = server.write().await;

                if let Err(err) = server.db.refresh_index(false).await {
                    error!("While refreshing index: {err}")
                }

                server
                    .db
                    .refresh(id)
                    .await
                    .map(|post| post.to_post_content())
            };

            match post {
                Ok(post) => {
                    let server = server.read().await;
                    server.post(req, post).await
                }
                Err(err) => Err(err.into()),
            }
        } else if req.method() == Method::GET && req_uri.starts_with("/public/") {
            let server = server.read().await;
            server.public(req).await
        } else if req.method() == Method::GET
            && req_uri.to_lowercase().as_str() == "/robots.txt"
        {
            Self::robots()
        } else {
            let server = server.read().await;
            server.not_found(req).await
        };

        match result {
            Ok(ok) => Ok(ok),
            Err(err) => {
                let response = Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("error: {err}")))?;
                Ok(response)
            }
        }
    }

    async fn public(&self, req: Request<Body>) -> Result<Response<Body>, Box<dyn Error>> {
        let subpath = req.uri().path().strip_prefix("/public/").unwrap();
        let path = self.public_dir.join(subpath);

        let is_pem = path.extension().map_or(false, |ext| ext == "pem");

        let is_id_rsa = path
            .file_name()
            .map_or(false, |name| name.to_string_lossy().contains("id_rsa"));

        // Always return 404 for symlinks, files ending in .pem, and files containing id_rsa
        if is_pem || is_id_rsa || path.is_symlink() {
            return self.not_found(req).await;
        }

        let mut file = match File::open(&path).await {
            Ok(file) => file,
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    return self.not_found(req).await;
                } else {
                    return Err(err.into());
                }
            }
        };

        let post_last_modified = file
            .metadata()
            .await
            .and_then(|meta| meta.modified())
            .map(|lm| DateTime::<Local>::from(lm))
            .ok();

        let cache_valid = post_last_modified
            .as_ref()
            .map_or(false, |timestamp| req.headers().is_cache_valid(timestamp));

        if cache_valid {
            return Ok(Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())?);
        }

        let mut body = vec![];
        file.read_to_end(&mut body).await?;

        let resp = Response::builder()
            .status(StatusCode::OK)
            .header(CACHE_CONTROL, "max-age=3600");

        let resp = if let Some(lm) = post_last_modified {
            resp.header(LAST_MODIFIED, lm.to_rfc2822())
        } else {
            resp
        };

        Ok(resp.body(Body::from(body))?)
    }

    async fn index(
        &self,
        req: Request<Body>,
        content: PostContent,
    ) -> Result<Response<Body>, Box<dyn Error>> {
        if req.headers().is_cache_valid(&self.db.index_updated()) {
            return Ok(Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())?);
        }

        let canonical_url = self.db.site_url().clone();

        let req_url = if req.uri().host().is_some() {
            Url::parse(&req.uri().to_string())?
        } else {
            let mut req_url = canonical_url.clone();
            req_url.set_path(req.uri().path());
            req_url.set_query(req.uri().query());
            req_url
        };

        let last_modified = content.last_modified().to_rfc2822();
        let page = req_url
            .query_pairs()
            .filter_map(|(k, v)| {
                if k == "p" {
                    Some(v.parse::<usize>())
                } else {
                    None
                }
            })
            .nth(0)
            .unwrap_or(Ok(0));

        let page = match page {
            Ok(page) => page,
            Err(err) => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from(format!("error: {err}")))?)
            }
        };

        let nposts = self.db.all_posts().count();

        let posts = self
            .db
            .all_posts()
            .sorted_by(|a, b| b.cmp_published(a))
            // .sorted_by_key(|p| p.updated())
            .skip(page * self.index_page_len)
            .take(self.index_page_len)
            .map(|post| post.to_post_meta())
            .collect_vec();

        let is_end = nposts <= self.index_page_len * (page + 1);

        let vdom = VirtualDom::new_with_props(
            view::index,
            IndexProps {
                posts,
                page,
                is_end,
                content,
                canonical_url,
            },
        );
        let body = util::render_html(vdom, self.db.lang());

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CACHE_CONTROL, "max-age=3600")
            .header(LAST_MODIFIED, last_modified)
            .header(CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(body))?)
    }

    async fn post(
        &self,
        req: Request<Body>,
        post: PostContent,
    ) -> Result<Response<Body>, Box<dyn Error>> {
        if req.headers().is_cache_valid(&post.last_modified()) {
            return Ok(Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())?);
        }

        let site_title = self.db.site_title().to_string();
        let mut canonical_url = self.db.site_url().clone();
        canonical_url.set_path(req.uri().path());
        canonical_url.set_query(req.uri().query());
        let last_modified = post.last_modified().to_rfc2822();
        let twitter_link = self.db.twitter_link(&post.id)?;

        let vdom = VirtualDom::new_with_props(
            view::post,
            PostProps {
                post,
                site_title,
                canonical_url,
                twitter_link,
            },
        );
        let body = util::render_html(vdom, self.db.lang());

        let cache_control = format!("max-age={}", self.db.ttl().as_secs());

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CACHE_CONTROL, cache_control)
            .header(LAST_MODIFIED, last_modified)
            .header(CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(body))?)
    }

    async fn rss(&self, req: Request<Body>) -> Result<Response<Body>, Box<dyn Error>> {
        if req.headers().is_cache_valid(&self.db.index_updated()) {
            return Ok(Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())?);
        }

        let headers = req.headers();
        let if_modified_since = headers.if_modified_since();
        let deltas_supported = headers.accepted_manipulations()
            .map_or(false, |am| am.includes_feed());

        let since = if deltas_supported {
            if_modified_since.as_ref()
                .map(|ifs| ifs.as_datetime())
        } else {
            None
        };

        let include_content = match self.rss_content {
            RssContent::Never => false,
            RssContent::Always => true,
            RssContent::SupportsDeltas => deltas_supported,
        };

        let rss = self.db.get_rss(since, include_content, 25).build();
        let last_modified = self.db.index_updated().to_rfc2822();

        debug!("Sending {} items", rss.items.len());
    
        let cache_control = format!("im, max-age={}", self.db.ttl().as_secs());

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CACHE_CONTROL, cache_control)
            .header(LAST_MODIFIED, last_modified)
            .header(CONTENT_TYPE, "text/xml; charset=utf-8")
            .header(VARY, "A-IM, If-Modified-Since")
            .body(Body::from(rss.to_string()))?)
    }

    fn robots() -> Result<Response<Body>, Box<dyn Error>> {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from(ROBOTS_TXT))?)
    }

    async fn not_found(&self, req: Request<Body>) -> Result<Response<Body>, Box<dyn Error>> {
        let path = req.uri().clone();
        let method = req.method().clone();

        let vdom = VirtualDom::new_with_props(view::not_found, NotFoundProps { path, method });
        let body = util::render_html(vdom, self.db.lang());

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(body))?)
    }
}
