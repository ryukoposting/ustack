//! `serve` command handler.

use chrono::{DateTime, Local, Utc};
use clap::Parser;
use dioxus::prelude::*;
use hyper::{
    header::{CACHE_CONTROL, CONTENT_TYPE, LAST_MODIFIED},
    server::conn::AddrStream,
    service::service_fn,
    Body, Method, Request, Response, StatusCode,
};
use itertools::Itertools;
use log::{debug, info, warn, LevelFilter};
use std::{
    convert::Infallible, env, error::Error, io::ErrorKind, net::SocketAddr, num::NonZeroUsize,
    path::PathBuf, sync::Arc,
};
use tokio::{fs::File, io::AsyncReadExt, sync::RwLock};
use url::Url;

use crate::{
    util::{
        self,
        db::{PostContent, PostDb},
    },
    view::{self, IndexProps, NotFoundProps, PostProps},
};

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
}

struct Server {
    db: PostDb,
    address: SocketAddr,
    index_page_len: usize,
    public_dir: PathBuf,
}

const ROBOTS_TXT: &str = include_str!("res/robots.txt");

impl Serve {
    pub fn directory(&self) -> Result<PathBuf, std::io::Error> {
        self.directory
            .as_ref()
            .map_or_else(|| env::current_dir(), |path| dunce::canonicalize(path))
    }

    fn into_server(self) -> Result<Server, std::io::Error> {
        let dir = self.directory()?;
        let posts_dir = dir.join("posts");
        let public_dir = dir.join("public");

        let db = PostDb::new(posts_dir, self.cache_ttl)?;

        let server = Server {
            db,
            address: self.address,
            index_page_len: self.index_page_len.into(),
            public_dir,
        };
        Ok(server)
    }

    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let address = self.address.clone();
        let server = Arc::from(RwLock::new(self.into_server()?));

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

        let result = if req.method() == Method::GET && req.uri().path() == "/" {
            let index = {
                let mut server = server.write().await;
                server
                    .db
                    .refresh_index(true)
                    .await
                    .map(|post| post.to_post_content(None))
            };

            match index {
                Ok(index) => server.read().await.index(req, index).await,
                Err(err) => Err(err.into()),
            }
        } else if req.method() == Method::GET && req.uri().path().starts_with("/p/") {
            let post = {
                let id = req.uri().path().split('/').nth(2).unwrap_or("");
                let mut server = server.write().await;

                let index = server.db.refresh_index(false).await
                    .map(|index| index.canonical());

                let index_canonical = match index {
                    Err(err) => {
                        warn!("While refreshing index: {err}");
                        None
                    }
                    Ok(canonical) => canonical.map(|s| s.to_owned())
                };

                server
                    .db
                    .refresh(id)
                    .await
                    .map(|post| post.to_post_content(index_canonical.as_deref()))
            };

            match post {
                Ok(post) => {
                    let server = server.read().await;
                    server.post(req, post).await
                }
                Err(err) => Err(err.into()),
            }
        } else if req.method() == Method::GET && req.uri().path().starts_with("/public/") {
            let server = server.read().await;
            server.public(req).await
        } else if req.method() == Method::GET && req.uri().path().to_lowercase().as_str() == "/robots.txt" {
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

        let last_modified = file
            .metadata()
            .await
            .and_then(|meta| meta.modified())
            .map(|lm| DateTime::<Local>::from(lm))
            .ok();

        let cache_valid = last_modified
            .as_ref()
            .map_or(false, |timestamp| util::cache_valid(&req, timestamp));

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

        let resp = if let Some(lm) = last_modified {
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
        if util::cache_valid(&req, &self.db.index_updated()) {
            return Ok(Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())?);
        }

        let url = format!("http://dummy{}", req.uri());
        let last_modified = DateTime::<Utc>::from(content.timestamp).to_rfc2822();
        let page = Url::parse(&url).map(|url| {
            url.query_pairs()
                .filter_map(|(k, v)| {
                    if k == "p" {
                        Some(v.parse::<usize>())
                    } else {
                        None
                    }
                })
                .nth(0)
                .unwrap_or(Ok(0))
        })?;

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
            .sorted_by_key(|p| p.updated())
            .skip(page * self.index_page_len)
            .take(self.index_page_len)
            .map(|post| post.to_post_meta())
            .collect_vec();

        let is_end = nposts <= self.index_page_len * (page + 1);

        let mut vdom = VirtualDom::new_with_props(
            view::index,
            IndexProps {
                posts,
                page,
                is_end,
                content,
            },
        );
        let _ = vdom.rebuild();
        let body = dioxus_ssr::render(&vdom);

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
        if util::cache_valid(&req, &post.timestamp) {
            return Ok(Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Body::empty())?);
        }

        let site_title = self
            .db
            .site_title()
            .map_or_else(|| "Untitled Blog".to_string(), |title| title.to_string());
        let last_modified = DateTime::<Utc>::from(post.timestamp).to_rfc2822();
        let mut vdom = VirtualDom::new_with_props(view::post, PostProps { post, site_title });
        let _ = vdom.rebuild();
        let body = dioxus_ssr::render(&vdom);

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CACHE_CONTROL, "max-age=3600")
            .header(LAST_MODIFIED, last_modified)
            .header(CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(body))?)
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
        let mut vdom = VirtualDom::new_with_props(view::not_found, NotFoundProps { path, method });
        let _ = vdom.rebuild();

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(dioxus_ssr::render(&vdom)))?)
    }
}
