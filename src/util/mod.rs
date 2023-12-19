use chrono::{DateTime, TimeZone, FixedOffset};
use dioxus::prelude::VirtualDom;
use hyper::{Request, Body, header::{IF_MODIFIED_SINCE, CACHE_CONTROL}};
use url::Url;

pub mod db;
pub mod mydatetime;

pub struct IfModifiedSince(Option<DateTime<FixedOffset>>);
pub struct CacheControl<'a>(Option<&'a str>);

pub fn cache_valid<TZ>(req: &Request<Body>, timestamp: &DateTime<TZ>) -> bool
where
    TZ: TimeZone
{
    let cache_valid = IfModifiedSince::new(req).is_valid(timestamp);

    let no_cache = CacheControl::new(req).is_no_cache();

    cache_valid && !no_cache
}

pub fn render_html(mut vdom: VirtualDom, lang: &str) -> String {
    let _ = vdom.rebuild();
    let mut renderer = dioxus_ssr::Renderer::new();
    renderer.sanitize = true;
    let lang = html_escape::encode_unquoted_attribute(lang);
    format!("<!DOCTYPE html><html lang=\"{lang}\">{}</html>", renderer.render(&vdom))
}

pub fn render_base_part(url: &Url) -> String {
    let href = html_escape::encode_unquoted_attribute(url.as_str());
    format!("<base href=\"{href}\" />")
}

impl IfModifiedSince {
    pub fn new(req: &Request<Body>) -> Self {
        let ifm = req.headers()
            .get(IF_MODIFIED_SINCE)
            .and_then(|ifm| {
                let s = ifm.to_str().ok()?;
                DateTime::parse_from_rfc2822(s).ok()
            });
        Self(ifm)
    }

    pub fn is_valid<TZ>(&self, timestamp: &DateTime<TZ>) -> bool
    where
        TZ: TimeZone
    {
        self.0.map_or(false, |if_modified_since| {
            timestamp <= &(if_modified_since + chrono::Duration::seconds(1))
        })
    }

    pub fn as_datetime(&self) -> Option<&DateTime<FixedOffset>> {
        self.0.as_ref()
    }
}

impl<'a> CacheControl<'a> {
    pub fn new(req: &'a Request<Body>) -> Self {
        let cc = req.headers()
            .get(CACHE_CONTROL)
            .and_then(|cc| cc.to_str().ok());
        Self(cc)
    }

    pub fn is_no_cache(&self) -> bool {
        self.0.map_or(false, |cc| cc.contains("no-cache"))
    }
}
