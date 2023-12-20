use dioxus::prelude::VirtualDom;
use url::Url;

pub mod db;
pub mod mydatetime;
pub mod header_ext;

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
