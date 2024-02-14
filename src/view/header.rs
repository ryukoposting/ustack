use dioxus::prelude::*;

#[derive(Props)]
pub struct HeaderProps<'a> {
    pub site_title: &'a str,
    pub site_title_short: &'a str,
    #[props(!optional)]
    pub coffee_link: Option<&'a str>
}

pub fn site_header<'a>(cx: Scope<'a, HeaderProps<'a>>) -> Element<'a> {
    let coffee = cx.props.coffee_link
        .map(|c| cx.render(rsx! {
            a {
                href: "{c}",
                dangerous_inner_html: include_str!("../res/coffee.svg")
            }
        }));

    cx.render(rsx! {
        header {
            a {
                href: "/",
                h1 {
                    class: "site-title",
                    "{cx.props.site_title}"
                }
                h1 {
                    class: "short-title",
                    "{cx.props.site_title_short}"
                }
            }

            nav {
                a {
                    href: "/rss",
                    dangerous_inner_html: include_str!("../res/rss-icon.svg")
                }
                coffee
            }
        }
    })
}
