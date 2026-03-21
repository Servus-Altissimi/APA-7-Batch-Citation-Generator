use dioxus::prelude::*;
use dioxus::document::eval;
use crate::types::OutputFormat;

#[component]
pub fn OutputSec(
    output: Signal<String>,
    fmt: Signal<OutputFormat>,
    hovered: Signal<bool>,
    copied: Signal<bool>,
) -> Element {
    let on_copy = move |_| {
        let text    = output.read().clone();
        let is_html = *fmt.read() == OutputFormat::RichText;
        let escaped = text.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
        spawn(async move {
            let js = if is_html {
                format!(
                    "const b = new Blob([`{}`], {{type:'text/html'}});\
await navigator.clipboard.write([new ClipboardItem({{'text/html':b}})]);",
                        escaped
                )
            } else {
                format!("await navigator.clipboard.writeText(`{}`);", escaped)
            };
            let _ = eval(&js).await;
            copied.set(true);
        });
    };

    let has_output = !output.read().is_empty();

    rsx! {
        if has_output {
            p { class: "label",
                "Output: "
                code { { fmt.read().extension() } }
            }
            div {
                class: "output-wrap",
                onmouseenter: move |_| hovered.set(true),
                onmouseleave: move |_| { hovered.set(false); copied.set(false); },
                textarea {
                    class: "output-area",
                    rows: "18",
                    readonly: true,
                    value: "{output}",
                }
                button {
                    class: if *copied.read() { "copy-btn copied" } else { "copy-btn" },
                    onclick: on_copy,
                    if *copied.read() { "Copied" } else { "Copy" }
                }
            }
        }
    }
}
