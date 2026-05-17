//! External-reference scanners for the audit-detail endpoint — split from
//! `routes_audit_detail.rs` to keep that file under the 500-LOC budget.
//! Scans the output body for stable system identifiers (Stripe IDs, Slack
//! archive URLs, GitHub issues/PRs, Linear tickets, etc.) and returns a
//! capped list of `{system, id, deep_link}` rows.

use serde_json::{json, Value};

/// Per-output cap on the number of external references emitted by the
/// pre-scan; bounds compute and response size when an adversarial /
/// unusually-large output body trips many matches.
const MAX_EXTERNAL_REFS: usize = 30;

const URL_SCANNERS: &[(&str, &str)] = &[
    ("salesforce", "salesforce.com/"),
    ("hubspot", "hubspot.com/"),
    ("zendesk", ".zendesk.com/agent/tickets/"),
    ("jira", ".atlassian.net/browse/"),
];

pub(super) fn build_external_refs(payload: &Value) -> Value {
    let body_text = match payload.get("output_body") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => return Value::Array(Vec::new()),
    };
    let urls = iter_urls(&body_text);
    let mut hits: Vec<Value> = Vec::new();
    scan_stripe(&body_text, &mut hits);
    scan_slack(&urls, &mut hits);
    scan_github(&urls, &mut hits);
    for token in iter_uppercase_dashed_ids(&body_text) {
        hits.push(ref_entry(
            "linear",
            &token,
            &format!("https://linear.app/issue/{token}"),
        ));
    }
    for (system, marker) in URL_SCANNERS {
        scan_url_substring(&urls, system, marker, &mut hits);
    }
    hits.truncate(MAX_EXTERNAL_REFS);
    Value::Array(hits)
}

fn scan_stripe(text: &str, out: &mut Vec<Value>) {
    static PREFIXES: &[(&str, &str)] = &[
        ("re_", "refunds"),
        ("ch_", "payments"),
        ("cus_", "customers"),
        ("pi_", "payments"),
        ("in_", "invoices"),
        ("sub_", "subscriptions"),
    ];
    for (prefix, path) in PREFIXES {
        for token in matches_with_prefix(text, prefix) {
            let url = format!("https://dashboard.stripe.com/{path}/{token}");
            out.push(ref_entry("stripe", &token, &url));
        }
    }
}

fn scan_slack(urls: &[String], out: &mut Vec<Value>) {
    for url in urls {
        if let Some(rest) = url.strip_prefix("https://") {
            if let Some(tail) = rest.split_once(".slack.com/archives/").map(|(_, t)| t) {
                let id: String = tail
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-'))
                    .collect();
                if !id.is_empty() {
                    out.push(ref_entry("slack", &id, url));
                }
            }
        }
    }
}

fn scan_github(urls: &[String], out: &mut Vec<Value>) {
    for url in urls {
        let Some(rest) = url.strip_prefix("https://github.com/") else {
            continue;
        };
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 4 && (parts[2] == "issues" || parts[2] == "pull") {
            let id = format!("{}/{}#{}", parts[0], parts[1], parts[3]);
            out.push(ref_entry("github", &id, url));
        }
    }
}

fn scan_url_substring(urls: &[String], system: &str, marker: &str, out: &mut Vec<Value>) {
    for url in urls {
        if url.contains(marker) {
            let id = url.rsplit('/').next().unwrap_or(url).to_string();
            out.push(ref_entry(system, &id, url));
        }
    }
}

fn ref_entry(system: &str, id: &str, deep_link: &str) -> Value {
    json!({ "system": system, "id": id, "deep_link": deep_link })
}

/// Iterate ASCII tokens that begin with `prefix` and run until the next
/// non-`[A-Za-z0-9_]` byte. Used by the Stripe scanner.
fn matches_with_prefix(text: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    let mut i = 0;
    while i + prefix_bytes.len() <= bytes.len() {
        let starts = bytes[i..i + prefix_bytes.len()] == *prefix_bytes;
        let boundary = i == 0 || !is_id_byte(bytes[i - 1]);
        if starts && boundary {
            let mut j = i + prefix_bytes.len();
            while j < bytes.len() && is_id_byte(bytes[j]) {
                j += 1;
            }
            if j > i + prefix_bytes.len() {
                out.push(std::str::from_utf8(&bytes[i..j]).unwrap_or("").to_string());
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

fn is_id_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn iter_urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in text.split_ascii_whitespace() {
        let cleaned = token.trim_end_matches([',', '.', ')', ']', '"', '\'']);
        if cleaned.starts_with("https://") || cleaned.starts_with("http://") {
            out.push(cleaned.to_string());
        }
    }
    out
}

fn iter_uppercase_dashed_ids(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_uppercase() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_uppercase() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'-' && start + 2 <= i {
            let dash = i;
            i += 1;
            let num_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > num_start {
                out.push(
                    std::str::from_utf8(&bytes[start..i])
                        .unwrap_or("")
                        .to_string(),
                );
            } else {
                i = dash;
            }
        }
    }
    out
}
