use dioxus::prelude::*;
use crate::types::OutputFormat;
use crate::history::{HistoryEntry, fmt_date, load_history, save_history, cut_input_preview};

const PAGE_SIZE: usize = 5;

#[component]
pub fn HistorySec(
    hist_open: Signal<bool>,
    hist_entries: Signal<Vec<HistoryEntry>>,
    hist_page: Signal<usize>,
    hist_loading: Signal<bool>,
    output: Signal<String>,
    doi_input: Signal<String>,
    fmt: Signal<OutputFormat>,
    copied: Signal<bool>,
) -> Element {
    // Rload from localStorage
    let toggle_history = move |_| {
        let opening = !*hist_open.read();
        hist_open.set(opening);
        hist_page.set(0);
        if opening {
            spawn(async move {
                hist_loading.set(true);
                hist_entries.set(load_history().await);
                hist_loading.set(false);
            });
        }
    };

    let on_clear = move |_| {
        spawn(async move {
            hist_entries.set(Vec::new());
            save_history(&[]).await;
        });
    };

    let on_prev = move |_| {
        let p = *hist_page.read();
        if p > 0 { hist_page.set(p - 1); }
    };
    let on_next = move |_| {
        let total = hist_entries.read().len();
        let pages = total.saturating_sub(1) / PAGE_SIZE + 1;
        let p = *hist_page.read();
        if p + 1 < pages { hist_page.set(p + 1); }
    };

    // calc page slice + grouping before rsx! to keep the macro clean
    let h_open    = *hist_open.read();
    let h_loading = *hist_loading.read();
    let h_total   = hist_entries.read().len();
    let h_page    = *hist_page.read();
    let h_pages   = if h_total == 0 { 1 } else { (h_total + PAGE_SIZE - 1) / PAGE_SIZE };
    let h_start   = h_page * PAGE_SIZE;
    let h_end     = (h_start + PAGE_SIZE).min(h_total);
    let h_slice: Vec<HistoryEntry> = hist_entries.read()[h_start..h_end].to_vec();

    let mut h_groups: Vec<(String, Vec<HistoryEntry>)> = Vec::new();
    for entry in h_slice {
        if h_groups.last().map(|(d, _)| d == &entry.date_str).unwrap_or(false) {
            h_groups.last_mut().unwrap().1.push(entry);
        } else {
            h_groups.push((entry.date_str.clone(), vec![entry]));
        }
    }

    let h_badge   = (h_total > 0).then(|| h_total.to_string());
    let h_summary = if h_total == 0 {
        "No history yet".to_string()
    } else {
        let s = if h_total == 1 { "" } else { "es" };
        format!("{} saved batch{}", h_total, s)
    };

    rsx! {
        div { class: "history-section",
            button {
                class: if h_open { "history-toggle history-toggle--open" } else { "history-toggle" },
                onclick: toggle_history,
                if h_open { "▲" } else { "▼" }
                " History"
                if let Some(badge) = h_badge {
                    span { class: "history-badge", "{badge}" }
                }
            }

            if h_open {
                div { class: "history-panel",
                    div { class: "history-panel__header",
                        span { class: "history-panel__title", "{h_summary}" }
                        if h_total > 0 {
                            button { class: "history-clear-btn", onclick: on_clear, "Clear all" }
                        }
                    }

                    if h_loading {
                        p { class: "history-loading", "Loading history…" }
                    } else if h_total == 0 {
                        p { class: "history-empty", "Generate citations to build up a history." }
                    } else {
                        div { class: "history-groups",
                            { h_groups.into_iter().map(|(date, group_entries)| {
                                let date_label = fmt_date(&date);
                                rsx! {
                                    div { class: "history-date-group", key: "{date}",
                                    h3 { class: "history-date-label", "{date_label}" }
                                    { group_entries.into_iter().map(|entry| {
                                        let fmt_short = {
                                            let lbl = OutputFormat::from_index(entry.format_index).label();
                                            lbl.split('(').next().unwrap_or(lbl).trim().to_string()
                                        };
                                        let doi_word = if entry.doi_inputs.len() == 1 { "DOI" } else { "DOIs" };
                                        let counts = format!(
                                            "{} {} - {} OK{}",
                                            entry.doi_inputs.len(), doi_word, entry.success,
                                                             if entry.failed > 0 { format!(", {} failed", entry.failed) }
                                                             else { String::new() }
                                        );
                                        let preview  = cut_input_preview(&entry.doi_inputs);
                                        let has_fail = entry.failed > 0;
                                        let entry_id = entry.id.clone();
                                        let e_restore = entry.clone();
                                        let e_del     = entry.clone();

                                        rsx! {
                                            div { class: "history-entry", key: "{entry_id}",
                                            div { class: "history-entry__meta",
                                                span { class: "history-entry__time", "{entry.time_str}" }
                                                span { class: "history-entry__fmt-badge", "{fmt_short}" }
                                                span {
                                                    class: if has_fail {
                                                        "history-entry__counts history-entry__counts--warn"
                                                    } else {
                                                        "history-entry__counts"
                                                    },
                                                    "{counts}"
                                                }
                                            }
                                            div { class: "history-entry__preview", "{preview}" }
                                            div { class: "history-entry__actions",
                                                button {
                                                    class: "history-restore-btn",
                                                    title: "Load this batch into the editor",
                                                    onclick: move |_| {
                                                        output.set(e_restore.output.clone());
                                                        doi_input.set(e_restore.doi_inputs.join("\n"));
                                                        fmt.set(OutputFormat::from_index(e_restore.format_index));
                                                        copied.set(false);
                                                        hist_open.set(false);
                                                    },
                                                    "Restore"
                                                }
                                                button {
                                                    class: "history-delete-btn",
                                                    title: "Delete this entry",
                                                    onclick: move |_| {
                                                        let del_id = e_del.id.clone();
                                                        spawn(async move {
                                                            let new_entries: Vec<HistoryEntry> = {
                                                                let mut h = hist_entries.write();
                                                                h.retain(|e| e.id != del_id);
                                                                h.clone()
                                                            };
                                                            save_history(&new_entries).await;
                                                            // Clamp page index if last page became empty
                                                            let new_pages = if new_entries.is_empty() { 1 }
                                                            else { (new_entries.len() + PAGE_SIZE - 1) / PAGE_SIZE };
                                                            if *hist_page.read() >= new_pages {
                                                                hist_page.set(new_pages.saturating_sub(1));
                                                            }
                                                        });
                                                    },
                                                    "×"
                                                }
                                            }
                                            }
                                        }
                                    })}
                                    }
                                }
                            })}
                        }

                        if h_pages > 1 {
                            div { class: "history-pagination",
                                button {
                                    class: "history-page-btn",
                                    disabled: h_page == 0,
                                    onclick: on_prev,
                                    "← Prev"
                                }
                                span { class: "history-page-info",
                                    "Page {h_page + 1} of {h_pages}"
                                }
                                button {
                                    class: "history-page-btn",
                                    disabled: h_page + 1 >= h_pages,
                                    onclick: on_next,
                                    "Next →"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
