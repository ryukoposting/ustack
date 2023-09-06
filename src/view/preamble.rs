use dioxus::prelude::*;
use url::Url;

#[derive(Props)]
pub struct PreambleProps<'a> {
    title: &'a str,
    url: &'a Url,
    highlight: bool,
    #[props(!optional)]
    author: Option<&'a str>,
    #[props(!optional)]
    summary: Option<&'a str>,
    tags: &'a Vec<String>,
}

pub fn preamble<'a>(cx: Scope<'a, PreambleProps<'a>>) -> Element<'a> {
    let highlight = if cx.props.highlight {
        cx.render(rsx! {
            link {
                rel: "stylesheet",
                href: "https://unpkg.com/@highlightjs/cdn-assets@11.8.0/styles/vs2015.min.css"
            }
            script {
                src: "https://unpkg.com/@highlightjs/cdn-assets@11.8.0/highlight.min.js"
            }
            script {
                "hljs.highlightAll();"
            }
        })
    } else {
        None
    };

    let author = cx.props.author.and_then(|author| cx.render(rsx! {
        meta { name: "author", content: "{author}" }
    }));

    let summary = cx.props.summary.and_then(|summary| cx.render(rsx! {
        meta { name: "description", content: "{summary}" }
    }));

    let keywords = if cx.props.tags.len() > 0 {
        let keywords = cx.props.tags.join(", ");
        cx.render(rsx! {
            meta { name: "keywords", content: "{keywords}" }
        })
    } else {
        None
    };

    cx.render(rsx! {
        head {
            meta { charset: "utf-8" }
            meta { name: "viewport", content: "width=device-width,initial-scale=1" }
            title { "{cx.props.title}" }
            meta { name: "twitter:card", content: "summary" }
            link { rel: "canonical", href: "{cx.props.url}" }
            link { rel: "icon", href: "/public/favicon.png" }
            link { rel: "apple-touch-icon", href: "/public/favicon.png" }
            author
            summary
            keywords
            highlight
            link {
                rel: "stylesheet",
                href: "/public/styles.css"
            }
        }
    })
}
