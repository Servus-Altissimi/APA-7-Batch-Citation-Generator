use serde_json::Value;

pub fn extract_doi_from_string(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if &bytes[i..i + 3] == b"10." {
            let rest   = &s[i + 3..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 4 {
                let after = &rest[digits.len()..];
                if after.starts_with('/') {
                    let suffix: String = after[1..].chars()
                        .take_while(|&c| !c.is_whitespace()
                            && !matches!(c, '"' | '\'' | '<' | '>' | ',' | ')' | ']' | '}'))
                        .collect();
                    let suffix = suffix.trim_end_matches(|c| matches!(c, '.' | ';' | ':'));
                    if !suffix.is_empty() {
                        return Some(format!("10.{}/{}", digits, suffix));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

fn extract_meta_doi(html: &str, meta_name: &str) -> Option<String> {
    let lower   = html.to_lowercase();
    let needle  = format!("name=\"{}\"", meta_name.to_lowercase());
    let needle2 = format!("name='{}'",   meta_name.to_lowercase());
    let tag_start = lower.find(&needle).or_else(|| lower.find(&needle2))?;
    let open  = html[..tag_start].rfind('<').unwrap_or(tag_start);
    let close = html[tag_start..].find('>').map(|p| tag_start + p)?;
    let tag   = &html[open..=close];
    let tag_l = tag.to_lowercase();
    let ci    = tag_l.find("content=")? + 8;
    let after = &tag[ci..];
    let inner = if after.starts_with('"') {
        let end = after[1..].find('"').map(|p| p + 1)?;
        &after[1..end]
    } else if after.starts_with('\'') {
        let end = after[1..].find('\'').map(|p| p + 1)?;
        &after[1..end]
    } else { return None; };
    let candidate = inner.trim_start_matches("doi:").trim();
    if candidate.starts_with("10.") { Some(candidate.to_string()) } else { None }
}

pub fn normalize_doi(doi: &str) -> String {
    doi.trim()
       .trim_start_matches("https://doi.org/")
       .trim_start_matches("http://doi.org/")
       .trim_start_matches("doi:")
       .trim().to_string()
}

pub async fn resolve_to_doi(input: &str) -> Result<String, String> {
    let s = input.trim();
    if s.starts_with("10.") { return Ok(s.to_string()); }
    if s.starts_with("doi:") || s.starts_with("https://doi.org/") || s.starts_with("http://doi.org/") {
        return Ok(normalize_doi(s));
    }
    if s.starts_with("http://") || s.starts_with("https://") {
        if let Some(doi) = extract_doi_from_string(s) { return Ok(doi); }
        let html: String = reqwest::Client::new()
            .get(s)
            .header("User-Agent", "DOI-APA-Generator/2.0")
            .send().await.map_err(|e| format!("Fetch failed: {}", e))?
            .text().await.map_err(|e| format!("Read failed: {}", e))?;
        for name in &["citation_doi","dc.identifier","DC.identifier","dc.Identifier","bepress_citation_doi","rft_id"] {
            if let Some(doi) = extract_meta_doi(&html, name) { return Ok(doi); }
        }
        if let Some(doi) = extract_doi_from_string(&html) { return Ok(doi); }
        return Err(format!("No DOI found in page: {}", s));
    }
    Err(format!("Unrecognized input: {}", s))
}

pub async fn fetch_doi_metadata(doi: &str) -> Result<Value, String> {
    let doi = normalize_doi(doi);
    let client = reqwest::Client::new();
    let mut errors: Vec<String> = Vec::new();

    match client.get(format!("https://doi.org/{}", doi))
        .header("Accept", "application/vnd.citationstyles.csl+json")
        .header("User-Agent", "DOI-APA-Generator/2.0") // no need to do any obfuscation here
        .send().await
    {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(d) if d["DOI"].is_string() => return Ok(d),
            Ok(_)  => errors.push("doi.org: Invalid CSL-JSON".into()),
            Err(e) => errors.push(format!("doi.org: parse error: {}", e)),
        },
        Ok(r)  => errors.push(format!("doi.org: HTTP {}", r.status())),
        Err(e) => errors.push(format!("doi.org: {}", e)),
    }

    match client.get(format!("https://api.crossref.org/works/{}", doi))
        .header("Accept", "application/json").send().await
    {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(d) if !d["message"].is_null() => return Ok(d["message"].clone()),
            Ok(_)  => errors.push("CrossRef: Invalid response".into()),
            Err(e) => errors.push(format!("CrossRef: parse error: {}", e)),
        },
        Ok(r)  => errors.push(format!("CrossRef: HTTP {}", r.status())),
        Err(e) => errors.push(format!("CrossRef: {}", e)),
    }

    match client.get(format!("https://api.datacite.org/dois/{}", doi))
        .header("Accept", "application/json").send().await
    {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(data) => {
                if let Some(attrs) = data["data"]["attributes"].as_object() {
                    let title = attrs.get("titles").and_then(|t| t.as_array())
                        .and_then(|a| a.first()).and_then(|t| t["title"].as_str())
                        .unwrap_or("").to_string();
                    let container = attrs.get("container")
                        .and_then(|c| c["title"].as_str()).unwrap_or("").to_string();
                    let authors: Vec<Value> = attrs.get("creators")
                        .and_then(|c| c.as_array()).map(|creators| {
                            creators.iter().map(|c| serde_json::json!({
                                "family": c["familyName"].as_str().or_else(|| c["name"].as_str()).unwrap_or(""),
                                "given":  c["givenName"].as_str().unwrap_or(""),
                            })).collect()
                        }).unwrap_or_default();
                    let mut norm = serde_json::json!({
                        "DOI": doi, "author": authors, "title": title,
                        "container-title": container,
                        "volume": attrs.get("volume"), "issue": attrs.get("issue"),
                        "page":   attrs.get("page"),
                    });
                    if let Some(pub_) = attrs.get("published").and_then(|p| p.as_str()) {
                        let yr = &pub_[..4.min(pub_.len())];
                        norm["issued"] = serde_json::json!({ "date-parts": [[yr]] });
                    }
                    return Ok(norm);
                }
                errors.push("DataCite: Invalid response".into());
            }
            Err(e) => errors.push(format!("DataCite: parse error: {}", e)),
        },
        Ok(r)  => errors.push(format!("DataCite: HTTP {}", r.status())),
        Err(e) => errors.push(format!("DataCite: {}", e)),
    }

    Err(format!("All APIs failed:\n  {}", errors.join("\n  ")))
}
