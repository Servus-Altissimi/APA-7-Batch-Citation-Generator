use dioxus::document::eval;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct HistoryEntry {
    pub id:           String, // timestamp ms as string -> unique key
    pub timestamp_ms: i64,
    pub date_str:     String,
    pub time_str:     String,
    pub doi_inputs:   Vec<String>,
    pub output:       String,
    pub format_index: usize,
    pub success:      usize,
    pub failed:       usize,
}

const KEY: &str  = "apaciter_history";
const MAX: usize = 200;

// Format cleanly, amateu.
pub fn fmt_date(s: &str) -> String {
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 { return s.to_string(); }
    let month = match p[1] {
        "01"=>"January","02"=>"February","03"=>"March",    "04"=>"April",
        "05"=>"May",    "06"=>"June",    "07"=>"July",     "08"=>"August",
        "09"=>"September","10"=>"October","11"=>"November","12"=>"December",
        m => m,
    };
    format!("{} {}, {}", month, p[2].trim_start_matches('0'), p[0])
}

pub async fn load_history() -> Vec<HistoryEntry> {
    match eval(&format!("return localStorage.getItem('{}') || '[]'", KEY)).await {
        Ok(v) => serde_json::from_str(v.as_str().unwrap_or("[]")).unwrap_or_default(),
        Err(_) => vec![],
    }
}

pub async fn save_history(entries: &[HistoryEntry]) {
    let Ok(json) = serde_json::to_string(entries) else { return };
    let safe = json.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
    let _ = eval(&format!("localStorage.setItem('{}',`{}`);", KEY, safe)).await;
}

// Returns in local time.
pub async fn now_info() -> (i64, String, String) {
    let js = "const d=new Date(); return [\
        d.getTime(),\
        d.getFullYear()+'-'+String(d.getMonth()+1).padStart(2,'0')+'-'+String(d.getDate()).padStart(2,'0'),\
        String(d.getHours()).padStart(2,'0')+':'+String(d.getMinutes()).padStart(2,'0')\
    ];";
    match eval(js).await {
        Ok(v) => (
            v[0].as_i64().unwrap_or(0),
            v[1].as_str().unwrap_or("").to_string(),
            v[2].as_str().unwrap_or("").to_string(),
        ),
        Err(_) => (0, "Unknown".into(), String::new()),
    }
}

pub fn cut_input_preview(inputs: &[String]) -> String {
    let first = inputs.first().cloned().unwrap_or_default();
    let first = if first.len() > 52 { format!("{}…", &first[..52]) } else { first };
    if inputs.len() > 1 {
        format!("{} (+{} more)", first, inputs.len() - 1)

    } else { first }
}
