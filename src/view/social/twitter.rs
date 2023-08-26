use dioxus::prelude::*;
use url::Url;

#[derive(Props)]
pub struct TwitterShareProps<'a> {
    text: &'a str,
}

pub fn twitter_share<'a>(cx: Scope<'a, TwitterShareProps<'a>>) -> Element<'a> {
    let mut path = Url::parse("https://twitter.com/intent/tweet").unwrap();

    path.query_pairs_mut()
        .append_pair("text", cx.props.text);

    cx.render(rsx! {
        a {
            class: "share twitter",
            href: "{path}",
            rel: "nofollow",
            "Tweet"
        }
    })
}
