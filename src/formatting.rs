use serde_json::Value;
use crate::types::OutputFormat;

// LaTeX helpers
pub fn escape_latex(s: &str) -> String {
    s.replace('\\', "\\textbackslash{}")
        .replace('&',  "\\&")
        .replace('%',  "\\%")
        .replace('$',  "\\$")
        .replace('#',  "\\#")
        .replace('_',  "\\_")
        .replace('{',  "\\{")
        .replace('}',  "\\}")
        .replace('~',  "\\textasciitilde{}")
        .replace('^',  "\\textasciicircum{}")
}

fn format_authors_latex(authors: &Value) -> String {
    let arr = match authors.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return String::new(),
    };
    let mut fmt: Vec<String> = Vec::new();
    for a in arr {
        let family   = escape_latex(a["family"].as_str().unwrap_or(""));
        let initials = initials_from(a["given"].as_str().unwrap_or(""));
        if !family.is_empty() {
            fmt.push(if initials.is_empty() { family }
                     else { format!("{}, {}", family, initials) });
        }
    }
    join_authors_apa(&fmt, "\\& ")
}

// Shared helpers
fn format_authors_plain(authors: &Value) -> String {
    let arr = match authors.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return String::new(),
    };
    let mut fmt: Vec<String> = Vec::new();
    for a in arr {
        let family   = a["family"].as_str().unwrap_or("").to_string();
        let initials = initials_from(a["given"].as_str().unwrap_or(""));
        if !family.is_empty() {
            fmt.push(if initials.is_empty() { family }
                     else { format!("{}, {}", family, initials) });
        }
    }
    join_authors_apa(&fmt, "& ")
}

fn initials_from(given: &str) -> String {
    let mut out = String::new();
    for word in given.split_whitespace() {
        if let Some(c) = word.chars().next() {
            if c.is_alphabetic() {
                out.push(c.to_ascii_uppercase());
                out.push_str(". ");
            }
        }
    }
    out.trim_end().to_string()
}

fn join_authors_apa(v: &[String], ampersand: &str) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        2 => format!("{}, {} {}", v[0], ampersand, v[1]),
        _ => {
            let mut r = v[..v.len()-1].join(", ");
            r.push_str(&format!(", {} {}", ampersand, v.last().unwrap()));
            r
        }
    }
}

pub fn extract_year(date: &Value) -> String {
    if date.is_null() { return String::new(); }
    if let Some(y) = date["date-parts"].as_array()
        .and_then(|p| p.first()).and_then(|r| r.as_array()).and_then(|r| r.first())
    {
        if let Some(n) = y.as_u64() { return n.to_string(); }
        if let Some(s) = y.as_str() { return s.chars().take(4).collect(); }
    }
    if let Some(r) = date["raw"].as_str()      { if r.len() >= 4 { return r[..4].to_string(); } }
    if let Some(d) = date["date-time"].as_str() { if d.len() >= 4 { return d[..4].to_string(); } }
    String::new()
}

pub fn get_year(meta: &Value) -> String {
    [&meta["issued"], &meta["created"], &meta["published"]]
        .iter().find(|v| !v.is_null()).copied().map(extract_year).unwrap_or_default()
}

pub fn get_string(v: &Value) -> String {
    match v {
        Value::Array(a)  => a.first().and_then(|x| x.as_str()).unwrap_or("").to_string(),
        Value::String(s) => s.clone(),
        _                => String::new(),
    }
}

pub fn doi_key(doi: &str) -> String {
    doi.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
}

pub fn clean_doi(doi: &str) -> &str {
    doi.trim_start_matches("https://doi.org/")
       .trim_start_matches("http://doi.org/")
       .trim_start_matches("doi:")
       .trim()
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// Per-format-generators
fn generate_latex(meta: &Value, doi: &str) -> String {
    let mut p: Vec<String> = Vec::new();
    let auth = format_authors_latex(&meta["author"]);
    if !auth.is_empty() { p.push(format!("{}.", auth)); }
    let yr = get_year(meta);
    if !yr.is_empty() { p.push(format!("({}).", yr)); }
    let title = escape_latex(&get_string(&meta["title"]));
    let title = title.trim_end_matches('.').to_string();
    if !title.is_empty() { p.push(format!("\\textit{{{}}}.", title)); }
    let container = escape_latex(&get_string(&meta["container-title"]));
    if !container.is_empty() {
        let vol = meta["volume"].as_str().map(String::from)
            .or_else(|| meta["volume"].as_u64().map(|n| n.to_string()));
        let iss = meta["issue"].as_str().map(String::from)
            .or_else(|| meta["issue"].as_u64().map(|n| n.to_string()));
        let vi = match vol {
            Some(v) => match iss {
                Some(i) => format!("\\textit{{{}}}({})", escape_latex(&v), escape_latex(&i)),
                None    => format!("\\textit{{{}}}", escape_latex(&v)),
            },
            None => String::new(),
        };
        p.push(if vi.is_empty() { format!("\\textit{{{}}}.", container) }
               else             { format!("\\textit{{{}}}, {}.", container, vi) });
    }
    if let Some(pg) = meta["page"].as_str() {
        p.push(format!("{}.", escape_latex(pg).replace('-', "--")));
    }
    p.push(format!("\\url{{https://doi.org/{}}}", escape_latex(clean_doi(doi))));
    p.join(" ")
}

fn generate_bibtex(meta: &Value, doi: &str) -> String {
    let bib_authors = meta["author"].as_array().map(|arr| {
        arr.iter().map(|a| {
            let f = a["family"].as_str().unwrap_or("");
            let g = a["given"].as_str().unwrap_or("");
            if g.is_empty() { f.to_string() } else { format!("{}, {}", f, g) }
        }).collect::<Vec<_>>().join(" and ")
    }).unwrap_or_default();
    let title     = get_string(&meta["title"]);
    let journal   = get_string(&meta["container-title"]);
    let year      = get_year(meta);
    let volume    = meta["volume"].as_str().map(String::from)
                        .or_else(|| meta["volume"].as_u64().map(|n| n.to_string())).unwrap_or_default();
    let number    = meta["issue"].as_str().map(String::from)
                        .or_else(|| meta["issue"].as_u64().map(|n| n.to_string())).unwrap_or_default();
    let pages     = meta["page"].as_str().unwrap_or("").replace('-', "--");
    let doi_clean = clean_doi(doi).to_string();
    let key       = doi_key(&doi_clean);
    let mut lines = vec![format!("@article{{{},", key)];
    if !bib_authors.is_empty() { lines.push(format!("  author  = {{{}}},", bib_authors)); }
    if !title.is_empty()       { lines.push(format!("  title   = {{{{{}}}}},", title)); }
    if !journal.is_empty()     { lines.push(format!("  journal = {{{}}},", journal)); }
    if !year.is_empty()        { lines.push(format!("  year    = {{{}}},", year)); }
    if !volume.is_empty()      { lines.push(format!("  volume  = {{{}}},", volume)); }
    if !number.is_empty()      { lines.push(format!("  number  = {{{}}},", number)); }
    if !pages.is_empty()       { lines.push(format!("  pages   = {{{}}},", pages)); }
    lines.push(format!("  doi     = {{{}}},", doi_clean));
    lines.push(format!("  url     = {{https://doi.org/{}}},", doi_clean));
    lines.push("}".to_string());
    lines.join("\n")
}

fn generate_plain(meta: &Value, doi: &str) -> String {
    let mut p: Vec<String> = Vec::new();
    let auth = format_authors_plain(&meta["author"]);
    if !auth.is_empty() { p.push(format!("{}.", auth)); }
    let yr = get_year(meta);
    if !yr.is_empty() { p.push(format!("({}).", yr)); }
    let title = get_string(&meta["title"]).trim_end_matches('.').to_string();
    if !title.is_empty() { p.push(format!("{}.", title)); }
    let container = get_string(&meta["container-title"]);
    if !container.is_empty() {
        let vol = meta["volume"].as_str().map(String::from)
            .or_else(|| meta["volume"].as_u64().map(|n| n.to_string()));
        let iss = meta["issue"].as_str().map(String::from)
            .or_else(|| meta["issue"].as_u64().map(|n| n.to_string()));
        let vi = match vol {
            Some(v) => match iss { Some(i) => format!("{}({})", v, i), None => v },
            None => String::new(),
        };
        p.push(if vi.is_empty() { format!("{}.", container) }
               else             { format!("{}, {}.", container, vi) });
    }
    if let Some(pg) = meta["page"].as_str() { p.push(format!("{}.", pg)); }
    p.push(format!("https://doi.org/{}", clean_doi(doi)));
    p.join(" ")
}

fn generate_markdown(meta: &Value, doi: &str) -> String {
    let mut p: Vec<String> = Vec::new();
    let auth = format_authors_plain(&meta["author"]);
    if !auth.is_empty() { p.push(format!("{}.", auth)); }
    let yr = get_year(meta);
    if !yr.is_empty() { p.push(format!("({}).", yr)); }
    let title = get_string(&meta["title"]).trim_end_matches('.').to_string();
    if !title.is_empty() { p.push(format!("*{}*.", title)); }
    let container = get_string(&meta["container-title"]);
    if !container.is_empty() {
        let vol = meta["volume"].as_str().map(String::from)
            .or_else(|| meta["volume"].as_u64().map(|n| n.to_string()));
        let iss = meta["issue"].as_str().map(String::from)
            .or_else(|| meta["issue"].as_u64().map(|n| n.to_string()));
        let vi = match vol {
            Some(v) => match iss { Some(i) => format!("*{}*({})", v, i), None => format!("*{}*", v) },
            None => String::new(),
        };
        p.push(if vi.is_empty() { format!("*{}*.", container) }
               else             { format!("*{}*, {}.", container, vi) });
    }
    if let Some(pg) = meta["page"].as_str() { p.push(format!("{}.", pg)); }
    let d = clean_doi(doi);
    p.push(format!("[https://doi.org/{d}](https://doi.org/{d})"));
    p.join(" ")
}

fn generate_ris(meta: &Value, doi: &str) -> String {
    let mut lines: Vec<String> = vec!["TY  - JOUR".into()];
    if let Some(arr) = meta["author"].as_array() {
        for a in arr {
            let f = a["family"].as_str().unwrap_or("");
            let g = a["given"].as_str().unwrap_or("");
            let entry = if g.is_empty() { f.to_string() } else { format!("{}, {}", f, g) };
            if !entry.is_empty() { lines.push(format!("AU  - {}", entry)); }
        }
    }
    let title = get_string(&meta["title"]);
    if !title.is_empty() { lines.push(format!("TI  - {}", title)); }
    let journal = get_string(&meta["container-title"]);
    if !journal.is_empty() { lines.push(format!("JO  - {}", journal)); }
    let yr = get_year(meta);
    if !yr.is_empty() { lines.push(format!("PY  - {}", yr)); }
    let vol = meta["volume"].as_str().map(String::from)
        .or_else(|| meta["volume"].as_u64().map(|n| n.to_string())).unwrap_or_default();
    if !vol.is_empty() { lines.push(format!("VL  - {}", vol)); }
    let iss = meta["issue"].as_str().map(String::from)
        .or_else(|| meta["issue"].as_u64().map(|n| n.to_string())).unwrap_or_default();
    if !iss.is_empty() { lines.push(format!("IS  - {}", iss)); }
    if let Some(pg) = meta["page"].as_str() {
        let mut parts = pg.splitn(2, '-');
        if let Some(sp) = parts.next() { lines.push(format!("SP  - {}", sp.trim())); }
        if let Some(ep) = parts.next() { lines.push(format!("EP  - {}", ep.trim())); }
    }
    let d = clean_doi(doi);
    lines.push(format!("DO  - {}", d));
    lines.push(format!("UR  - https://doi.org/{}", d));
    lines.push("ER  -".into());
    lines.join("\n")
}

fn generate_richtext(meta: &Value, doi: &str) -> String {
    let mut p: Vec<String> = Vec::new();
    let auth = format_authors_plain(&meta["author"]);
    if !auth.is_empty() { p.push(format!("{}.", html_escape(&auth))); }
    let yr = get_year(meta);
    if !yr.is_empty() { p.push(format!("({}).", yr)); }
    let title = get_string(&meta["title"]).trim_end_matches('.').to_string();
    if !title.is_empty() { p.push(format!("<em>{}.</em>", html_escape(&title))); }
    let container = get_string(&meta["container-title"]);
    if !container.is_empty() {
        let vol = meta["volume"].as_str().map(String::from)
            .or_else(|| meta["volume"].as_u64().map(|n| n.to_string()));
        let iss = meta["issue"].as_str().map(String::from)
            .or_else(|| meta["issue"].as_u64().map(|n| n.to_string()));
        let vi = match vol {
            Some(v) => match iss {
                Some(i) => format!("<em>{}</em>({})", html_escape(&v), html_escape(&i)),
                None    => format!("<em>{}</em>", html_escape(&v)),
            },
            None => String::new(),
        };
        p.push(if vi.is_empty() {
            format!("<em>{}.</em>", html_escape(&container))
        } else {
            format!("<em>{}</em>, {}.", html_escape(&container), vi)
        });
    }
    if let Some(pg) = meta["page"].as_str() { p.push(format!("{}.", html_escape(pg))); }
    let d = clean_doi(doi);
    p.push(format!("<a href=\"https://doi.org/{d}\">https://doi.org/{d}</a>"));
    p.join(" ")
}

// Public entry points
pub fn build_entry(meta: &Value, doi: &str, fmt: OutputFormat) -> String {
    match fmt {
        OutputFormat::Latex     => format!("\\bibitem{{{}}}\n{}", doi_key(doi), generate_latex(meta, doi)),
        OutputFormat::BibTeX    => generate_bibtex(meta, doi),
        OutputFormat::PlainText => generate_plain(meta, doi),
        OutputFormat::Markdown  => generate_markdown(meta, doi),
        OutputFormat::Ris       => generate_ris(meta, doi),
        OutputFormat::RichText  => generate_richtext(meta, doi),
    }
}

pub fn wrap_output(entries: &[String], fmt: OutputFormat) -> String {
    match fmt {
        OutputFormat::Latex => format!(
            "\\begin{{thebibliography}}{{99}}\n\n{}\n\\end{{thebibliography}}\n",
            entries.join("\n")
        ),
        OutputFormat::BibTeX    => format!("{}\n", entries.join("\n\n")),
        OutputFormat::PlainText => entries.join("\n\n"),
        OutputFormat::Markdown  => entries.iter().map(|e| format!("- {}", e)).collect::<Vec<_>>().join("\n"),
        OutputFormat::Ris       => entries.join("\n\n"),
        OutputFormat::RichText  => format!(
            "<!DOCTYPE html><html><body style=\"font-family: Times New Roman, serif; font-size: 12pt;\">\
             <ol>\n{}\n</ol></body></html>",
            entries.iter().map(|e| format!("  <li>{}</li>", e)).collect::<Vec<_>>().join("\n")
        ),
    }
}
pub fn rerender(results: &[(String, Value)], fmt: OutputFormat) -> String {
    if results.is_empty() { return String::new(); }
    let entries: Vec<String> = results.iter()
        .map(|(doi, meta)| build_entry(meta, doi, fmt))
        .collect();
    wrap_output(&entries, fmt)
}
