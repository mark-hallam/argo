//! URL-query helpers shared by `routes.rs` dispatchers. Pulled
//! out so `routes.rs` stays under the 500-LOC ceiling. None of
//! these touch the store or perform IO — they're pure parsers.

/// Pull `limit=<N>` out of a path tail like
/// `messages?limit=10 HTTP/1.1`. Returns `None` if absent or
/// unparseable; caller treats `None` as "all rows".
pub(super) fn parse_limit(tail: &str) -> Option<usize> {
    parse_query_kv(tail, "limit").and_then(|v| v.parse().ok())
}

/// Pull `role=<s>` out of the path tail (e.g. `?role=user`).
pub(super) fn parse_role(tail: &str) -> Option<String> {
    parse_query_kv(tail, "role")
}

/// Pull `(q, limit, session)` out of a `/search?…` request.
/// `limit` defaults to 20 when missing or unparseable. `q` is
/// URL-decoded for `+` only — full % decoding is left to the
/// CLI client (the smoke test sends pre-decoded queries).
pub(super) fn parse_search_args(req: &str) -> (Option<String>, u32, Option<String>) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let q = parse_query_kv(tail, "q").map(|s| s.replace('+', " "));
    let limit = parse_query_kv(tail, "limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let session = parse_query_kv(tail, "session");
    (q, limit, session)
}

/// Pull `since-minutes=<N>` out of `GET <path>?since-minutes=N HTTP/1.1`.
pub(super) fn parse_since_minutes_from_get(req: &str, path: &str) -> Option<i64> {
    let prefix = format!("GET {path}?");
    let rest = req.strip_prefix(&prefix)?;
    let q = rest.split_once(' ').map(|(q, _)| q).unwrap_or(rest);
    for kv in q.split('&') {
        let (k, v) = kv.split_once('=')?;
        if k == "since-minutes" {
            return v.parse().ok();
        }
    }
    None
}

/// Look up a single `key=value` pair inside the query string of
/// `path?key1=val1&key2=val2 HTTP/1.1`. Stops at the first
/// space so trailing version-tokens don't leak into the value.
pub(super) fn parse_query_kv(tail: &str, key: &str) -> Option<String> {
    let q = tail.split_once('?')?.1;
    let q = q.split_once(' ').map(|(q, _)| q).unwrap_or(q);
    for kv in q.split('&') {
        let (k, v) = kv.split_once('=')?;
        if k == key {
            return Some(v.to_string());
        }
    }
    None
}
