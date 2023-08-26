use dioxus::prelude::*;
use url::Url;

use super::social;
use crate::util::db::PostContent;

#[derive(Props, PartialEq)]
pub struct PostProps {
    pub post: PostContent,
    pub site_title: String,
    #[props(!optional)]
    pub twitter_link: Option<Url>,
}

pub fn post(cx: Scope<PostProps>) -> Element {
    let timestamp = cx.props.post.last_modified().format("%A, %e %B %Y");
    let datetime = cx.props.post.last_modified().format("%F");
    let time_title = cx.props.post.last_modified().format("%e %B %Y");

    let address = if let Some(author) = &cx.props.post.metadata.author {
        cx.render(rsx! {
            address {
                class: "author",
                "Published by "
                a {
                    rel: "author",
                    "{author}"
                }
                " on "
                time {
                    datetime: "{datetime}",
                    title: "{time_title}",
                    "{timestamp}"
                }
            }
        })
    } else {
        cx.render(rsx! {
            address {
                class: "author",
                "Published on "
                time {
                    datetime: "{datetime}",
                    title: "{time_title}",
                    "{timestamp}"
                }
            }
        })
    };

    let twitter = cx
        .props
        .twitter_link
        .as_ref()
        .map(|link| cx.render(rsx! {
            social::twitter_share {
                text: "{link}"
            }
        }));

    cx.render(rsx! {
        super::preamble {
            title: &cx.props.post.metadata.title,
            highlight: cx.props.post.metadata.highlight,
            author: cx.props.post.metadata.author.as_deref(),
            summary: cx.props.post.metadata.summary.as_deref(),
            tags: &cx.props.post.metadata.tags
        }
        body {
            main {
                class: "post",
                header {
                    a {
                        href: "/",
                        h1 { "{cx.props.site_title}" }
                    }

                    nav {
                        a { href: "/", "Home" }
                    }
                }
                article {
                    header {
                        h1 { "{cx.props.post.metadata.title}" },
                        div {
                            class: "byline",
                            address,
                        }
                    }
                    div {
                        class: "article-body",
                        dangerous_inner_html: cx.props.post.body.as_str()
                    }
                }
            }
            footer {
                twitter
            }
        }
    })
}
