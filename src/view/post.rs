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
    let timestamp = cx.props.post.timestamp.format("%A, %e %B %Y");
    let datetime = cx.props.post.timestamp.format("%F");
    let time_title = cx.props.post.timestamp.format("%e %B %Y");

    let address = if let Some(author) = &cx.props.post.author {
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
            title: &cx.props.post.title,
            highlight: cx.props.post.highlight,
            author: cx.props.post.author.as_deref(),
            summary: cx.props.post.summary.as_deref(),
            tags: &cx.props.post.tags
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
                        h1 { "{cx.props.post.title}" },
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
