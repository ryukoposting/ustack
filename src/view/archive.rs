use dioxus::prelude::*;
use url::Url;

use crate::{model::Metadata, util::db::PostMeta};
use super::header;

pub struct ArchiveProps {
    pub posts: Vec<PostMeta>,
    pub canonical_url: Url,
    pub coffee_link: Option<Url>,
    pub site_title_short: String,
    pub metadata: Metadata
}

pub fn archive(cx: Scope<ArchiveProps>) -> Element {
    cx.render(rsx! {
        super::preamble {
            title: "Archive",
            highlight: false,
            author: cx.props.metadata.author.as_deref(),
            summary: None,
            url: &cx.props.canonical_url,
        }

        body {
            main {
                class: "archive",
                header::site_header {
                    site_title: &cx.props.metadata.title,
                    site_title_short: &cx.props.site_title_short,
                    coffee_link: cx.props.coffee_link.as_ref().map(|c| c.as_str())
                }

                section {
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
