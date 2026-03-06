//    __    ____   __      ___     ___  ____  _  _  ____  ____    __   ____  _____  ____
//   /__\  (  _ \ /__\    (__ )   / __)( ___)( \( )( ___)(  _ \  /__\ (_  _)(  _  )(  _ \
//  /(__)\  )___//(__)\    / /   ( (_-. )__)  )  (  )__)  )   / /(__)\  )(   )(_)(  )   /
// (__)(__)(__) (__)(__)  (_/     \___/(____)(_)\_)(____)(_)\_)(__)(__)(__) (_____)(_)\_)

// Copyright 2026 Servus Altissimi (Pseudonym)

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.


#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus::document::eval;
use serde_json::Value;

mod types;
mod formatting;
mod doi;

use types::OutputFormat;
use formatting::rerender;
use doi::{resolve_to_doi, fetch_doi_metadata};

const STYLE: Asset = asset!("/assets/style.scss");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut doi_input    = use_signal(|| String::new());
    let mut output       = use_signal(|| String::new());
    let mut loading      = use_signal(|| false);
    let mut status_lines = use_signal(|| Vec::<String>::new());
    // Stores (doi, metadata) for each successfully fetched entry, to rerender instantly when the format changes without re-fetching.
    let mut results: Signal<Vec<(String, Value)>> = use_signal(Vec::new);
    let mut fmt          = use_signal(|| OutputFormat::Latex);
    let mut hovered      = use_signal(|| false);
    let mut copied       = use_signal(|| false);

    let generate = move |_| {
        let input      = doi_input.read().clone();
        let chosen_fmt = *fmt.read();
        spawn(async move {
            loading.set(true);
            status_lines.set(Vec::new());
            output.set(String::new());
            results.set(Vec::new());

            let raw_lines: Vec<String> = input.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();

            if raw_lines.is_empty() {
                status_lines.write().push("No entries found.".into());
                loading.set(false);
                return;
            }

            status_lines.write().push(format!(
                "Found {} entr{}. Resolving DOIs…\n",
                raw_lines.len(),
                if raw_lines.len() == 1 { "y" } else { "ies" }
            ));

            let total = raw_lines.len();
            let mut success = 0usize;
            let mut failed  = 0usize;

            for (i, line) in raw_lines.iter().enumerate() {
                status_lines.write().push(format!("[{}/{}] {} …", i+1, total, line));

                let doi = match resolve_to_doi(line).await {
                    Ok(d) => d,
                    Err(e) => {
                        let mut s = status_lines.write();
                        if let Some(last) = s.last_mut() {
                            *last = format!("[{}/{}] {} NO DOI: {}", i+1, total, line, e);
                        }
                        failed += 1;
                        continue;
                    }
                };

                if doi != *line {
                    let mut s = status_lines.write();
                    if let Some(last) = s.last_mut() {
                        *last = format!("[{}/{}] {} → {} …", i+1, total, line, doi);
                    }
                }

                match fetch_doi_metadata(&doi).await {
                    Ok(meta) => {
                        results.write().push((doi.clone(), meta.clone()));
                        // Rerender full output after each successful fetch
                        let snap = results.read().clone();
                        output.set(rerender(&snap, chosen_fmt));

                        let mut s = status_lines.write();
                        if let Some(last) = s.last_mut() {
                            *last = format!("[{}/{}] {} OK", i+1, total, line);
                        }
                        success += 1;
                    }
                    Err(e) => {
                        let mut s = status_lines.write();
                        if let Some(last) = s.last_mut() {
                            *last = format!("[{}/{}] {} FAILED: {}", i+1, total, line, e);
                        }
                        failed += 1;
                    }
                }
            }

            status_lines.write().push(format!("\nDone. Success: {}  Failed: {}", success, failed));
            loading.set(false);
        });
    };

    // Called when the dropdown changes, immediately rerender stored results.
    let on_fmt_change = move |evt: Event<FormData>| {
        let idx: usize = evt.value().parse().unwrap_or(0);
        let new_fmt = OutputFormat::from_index(idx);
        fmt.set(new_fmt);
        let snap = results.read().clone();
        if !snap.is_empty() {
            output.set(rerender(&snap, new_fmt));
            copied.set(false);
        }
    };

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
    	document::Link {
        	rel: "icon",
        	r#type: "image/svg+xml",
        	href: asset!("/assets/icon.svg"),
    	}
    	
        document::Stylesheet { href: STYLE }
        div { class: "app",
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

            label { class: "field-label", "Input" }
            textarea {
                rows: "8",
                placeholder: "10.1007/s12220-025-02063-8\nhttps://link.springer.com/article/10.1007/s11051-025-06365-4\n# comment",
                value: "{doi_input}",
                oninput: move |e| doi_input.set(e.value()),
            }

            div { class: "controls-row",
                button {
                    disabled: *loading.read(),
                    onclick: generate,
                    if *loading.read() { "Fetching…" } else { "Generate" }
                }
                select {
                    onchange: on_fmt_change,
                    { OutputFormat::all().iter().enumerate().map(|(i, &f)| rsx! {
                        option {
                            value: "{i}",
                            selected: *fmt.read() == f,
                            { f.label() }
                        }
                    })}
                }
            }

            if !status_lines.read().is_empty() {
                pre { { status_lines.read().join("\n") } }
            }

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
