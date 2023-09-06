use chrono::{DateTime, TimeZone};
use dioxus::prelude::VirtualDom;
use hyper::{Request, Body, header::{IF_MODIFIED_SINCE, CACHE_CONTROL}};

pub mod db;

pub fn cache_valid<TZ>(req: &Request<Body>, timestamp: &DateTime<TZ>) -> bool
where
    TZ: TimeZone
{
    let cache_valid = req.headers()
        .get(IF_MODIFIED_SINCE)
        .and_then(|ifm| {
            let s = ifm.to_str().ok()?;
            DateTime::parse_from_rfc2822(s).ok()
        })
        .map_or(false, |if_modified_since| timestamp <= &(if_modified_since + chrono::Duration::seconds(1)));

    let no_cache = req.headers()
        .get(CACHE_CONTROL)
        .and_then(|cc| cc.to_str().ok())
        .map_or(false, |cc| cc.contains("no-cache"));

    cache_valid && !no_cache
}

pub fn render_html(mut vdom: VirtualDom, lang: &str) -> String {
    let _ = vdom.rebuild();
    let mut renderer = dioxus_ssr::Renderer::new();
    renderer.sanitize = true;
    let lang = html_escape::encode_unquoted_attribute(lang);
    format!("<!DOCTYPE html><html lang=\"{lang}\">{}</html>", renderer.render(&vdom))
}
