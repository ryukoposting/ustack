use dioxus::prelude::*;

#[derive(Props,PartialEq)]
pub struct PreambleProps {
    title: String,
    highlight: bool,
}

pub fn preamble(cx: Scope<PreambleProps>) -> Element {
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

    cx.render(rsx! {
        meta { charset: "utf-8" }
        meta { name: "viewport", content: "width=device-width,initial-scale=1" }
        title { "{cx.props.title}" }
        highlight
        link {
            rel: "stylesheet",
            href: "/public/styles.css"
        }
    })
}
