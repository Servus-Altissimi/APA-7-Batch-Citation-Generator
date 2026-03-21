use dioxus::prelude::*;

#[component]
pub fn AppHeader() -> Element {
    rsx! {
        header { class: "app-header",
            h1 {
                span { class: "accent", "APA 7 " }
                "Batch Citation Generator"
            }
            p { class: "subtitle",
                "Paste DOIs or article URLs; one per line. Comments start with "
                code { "#" }
                ". Accepts bare DOIs as: "
                code { "doi:…" }
                " or "
                code { "https://doi.org/…" }
                " or plain. Journal links, for example: Springer, Wiley, are also supported."
            }
        }
    }
}
