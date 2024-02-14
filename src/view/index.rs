use dioxus::prelude::*;
use url::Url;

use crate::util::db::{PostMeta, PostContent};
use super::header;

#[derive(Props, PartialEq)]
pub struct IndexProps {
    pub posts: Vec<PostMeta>,
    pub content: PostContent,
    pub canonical_url: Url,
    #[props(!optional)]
    pub coffee_link: Option<Url>,
    pub site_title_short: String,
}

pub fn index(cx: Scope<IndexProps>) -> Element {
    cx.render(rsx! {
        super::preamble {
            title: &cx.props.content.metadata.title,
            highlight: cx.props.content.metadata.highlight,
            author: cx.props.content.metadata.author.as_deref(),
            summary: cx.props.content.metadata.summary.as_deref(),
            tags: &cx.props.content.metadata.tags,
            url: &cx.props.canonical_url,
        }
        body {
            main {
                class: "index",
                header::site_header {
                    site_title: &cx.props.content.metadata.title,
                    site_title_short: &cx.props.site_title_short,
                    coffee_link: cx.props.coffee_link.as_ref().map(|c| c.as_str())
                }
                nav {
                    a {
                        href: "/archive",
                        "Archive"
                    },
                    a {
                        href: "/random",
                        "Random Post"
                    }
                }
                div {
                    class: "index-content",
                    dangerous_inner_html: "{cx.props.content.body}"
                }
                section {
                    h2 { "Recent Posts" }
                    ol {
                        for post in cx.props.posts.iter() {
                            li {
                                a {
                                    href: "/p/{post.id}",
                                    h3 { "{post.title}" }
                                }
                                post.summary.as_deref().unwrap_or_else(|| "")
                            }
                        }
                    }
                }
            }
        }
    })
}
