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
use serde_json::Value;

mod types;
mod formatting;
mod doi;
mod history;
mod components;

use types::OutputFormat;
use formatting::rerender;
use doi::{resolve_to_doi, fetch_doi_metadata};
use history::{HistoryEntry, now_info, save_history};
use components::{AppHeader, AppFooter, OutputSec, HistorySec};

const STYLE: Asset = asset!("/assets/style.scss");
const MAX_HIST: usize = 200;

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

    // history signals
    let mut hist_open:    Signal<bool>              = use_signal(|| false);
    let mut hist_entries: Signal<Vec<HistoryEntry>> = use_signal(Vec::new);
    let mut hist_page:    Signal<usize>             = use_signal(|| 0);
    let mut hist_loading: Signal<bool>              = use_signal(|| false);

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

            // Persist to history on any partial or full success
            if success > 0 {
                let (ts, date_str, time_str) = now_info().await;
                let entry = HistoryEntry {
                    id:           ts.to_string(),
              timestamp_ms: ts,
              date_str,
              time_str,
              doi_inputs:   raw_lines.clone(),
              output:       output.read().clone(),
              format_index: chosen_fmt.to_index(),
                  success,
              failed,
                };
                let snapshot: Vec<HistoryEntry> = {
                    let mut h = hist_entries.write();
                    h.insert(0, entry);
                    h.truncate(MAX_HIST);
                    h.clone()
                };
                save_history(&snapshot).await;
            }
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

    rsx! {
        document::Link {
            rel: "icon",
            r#type: "image/svg+xml",
            href: asset!("/assets/icon.svg"),
        }

        document::Stylesheet { href: STYLE }
        div { class: "app",
            AppHeader {}

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

            OutputSec { output, fmt, hovered, copied }
        }

        HistorySec {
            hist_open,
            hist_entries,
            hist_page,
            hist_loading,
            output,
            doi_input,
            fmt,
            copied,
        }

        AppFooter {}
    }
}
