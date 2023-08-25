use dioxus::prelude::*;

use crate::util::db::{PostMeta, PostContent};

#[derive(Props, PartialEq)]
pub struct IndexProps {
    pub posts: Vec<PostMeta>,
    pub page: usize,
    pub is_end: bool,
    pub content: PostContent,
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
            title: cx.props.content.title.to_string(),
            highlight: cx.props.content.highlight,
        }
        main {
            class: "index",
            header {
                a {
                    href: "/",
                    h1 { "{cx.props.content.title}" }
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
                h1 { "Recent Posts" }
                ol {
                    for post in cx.props.posts.iter() {
                        li {
                            a {
                                href: "/p/{post.id}",
                                h1 { "{post.title}" }
                            }

                            "{post.summary}"
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
    })
}
