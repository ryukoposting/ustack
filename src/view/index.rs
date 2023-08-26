use dioxus::prelude::*;
use url::Url;

use crate::util::db::{PostMeta, PostContent};

#[derive(Props, PartialEq)]
pub struct IndexProps {
    pub posts: Vec<PostMeta>,
    pub page: usize,
    pub is_end: bool,
    pub content: PostContent,
    pub canonical_url: Url,
}

pub fn index(cx: Scope<IndexProps>) -> Element {
    let back = if cx.props.page > 0 {
        cx.render(rsx! {
            a {
                href: "/?p={cx.props.page - 1}",
                "Previous"
            }
        })
    } else {
        cx.render(rsx! {
            a {
                class: "disabled",
                "Previous"
            }
        })
    };

    let forward = if !cx.props.is_end {
        cx.render(rsx! {
            a {
                href: "/?p={cx.props.page + 1}",
                "Next"
            }
        })
    } else {
        cx.render(rsx! {
            a {
                class: "disabled",
                "Next"
            }
        })
    };

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
                header {
                    a {
                        href: "/",
                        h1 { "{cx.props.content.metadata.title}" }
                    }

                    nav {
                        a { href: "/", "Home" }
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
                nav {
                    back
                    "Page {cx.props.page + 1}"
                    forward
                }
            }
        }
    })
}
