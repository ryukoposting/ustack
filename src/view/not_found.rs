use dioxus::prelude::*;
use hyper::{Uri, Method};

#[derive(Props, PartialEq)]
pub struct NotFoundProps {
    pub path: Uri,
    pub method: Method
}

pub fn not_found(cx: Scope<NotFoundProps>) -> Element {
    cx.render(rsx! {
        main {
            h1 { "404: Not Found" }
            p { "{cx.props.method} {cx.props.path}" }
        }
    })
}
