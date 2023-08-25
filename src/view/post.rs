use dioxus::prelude::*;

use crate::util::db::PostContent;

#[derive(Props, PartialEq)]
pub struct PostProps {
    pub post: PostContent,
    pub site_title: String,
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

    cx.render(rsx! {
        meta { name: "viewport", content: "width=device-width, initial-scale=1.0" }
        title { "{cx.props.post.title} | {cx.props.site_title}" }
        link {
            rel: "stylesheet",
            href: "/public/styles.css"
        }
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
    })
}
