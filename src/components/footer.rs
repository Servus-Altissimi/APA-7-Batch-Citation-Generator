use dioxus::prelude::*;

#[component]
pub fn AppFooter() -> Element {
    rsx! {
        footer { class: "app-footer",
            p {
                "A tool by "
                a { href: "https://constringo.com", target: "_blank", rel: "noopener noreferrer",
                    "constringo, "
                }
                "automating academic and research workflows."
            }
            p {
                "Questions or feedback? "
                a { href: "mailto:contact@constringo.com", "contact@constringo.com" }
            }
            p { class: "footer-copy", "© 2026 Constringo. All rights reserved." }
        }
    }
}
