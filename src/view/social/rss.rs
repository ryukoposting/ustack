use dioxus::prelude::*;
use url::Url;

#[derive(Props)]
pub struct RssProps<'a> {
    canonical_url: &'a Url
}

pub fn rss<'a>(cx: Scope<'a, RssProps<'a>>) -> Element<'a> {
    let mut url: Url = cx.props.canonical_url.clone();
    url.set_path("rss");

    cx.render(rsx! {
        a {
            class: "share rss",
            href: "{url}",
            "RSS"
        }
    })
}
