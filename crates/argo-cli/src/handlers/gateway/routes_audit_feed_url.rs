//! URL query-string helpers for `routes_audit_feed.rs`.
//!
//! Split out of `routes_audit_feed.rs` per HARD RULE 6 (500-LOC ceiling)
//! — see PR #2589 push it to 531 LOC; conductor mechanical extraction
//! restores headroom. The helpers are intentionally minimal and have no
//! dependency outside of `std`.

/// Pull a single value out of the request tail's query string.
pub(super) fn query_kv(tail: &str, key: &str) -> Option<String> {
    let head = tail.split_once(' ').map(|(p, _)| p).unwrap_or(tail);
    let qs = head.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| url_decode(v))
    })
}

/// Pull a comma-separated multi-value param, dropping empty entries.
pub(super) fn csv_param(tail: &str, key: &str) -> Vec<String> {
    query_kv(tail, key)
        .map(|raw| {
            raw.split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Minimal `+`→space + `%xx`→byte URL decoder; passes malformed
/// input through unchanged. Avoids a new dep for this one endpoint.
pub(super) fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '+' => out.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                    out.push(b as char);
                } else {
                    out.push('%');
                    out.push_str(&hex);
                }
            }
            other => out.push(other),
        }
    }
    out
}
