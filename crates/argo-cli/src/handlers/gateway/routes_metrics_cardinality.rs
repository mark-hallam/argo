//! `GET /metrics/cardinality` — JSON snapshot of the cardinality
//! tracker for ops triage. Returns
//! `{"families": {"<metric>": {"current_count", "cap", "capped_total"}}}`.
//!
//! Authentication: routed through the gateway's existing admin
//! boundary (mirrors the sibling `/metrics/json` route in
//! `routes_dispatch_get_admin.rs`). Adding a new
//! `Capability::OperatorReadOnly` variant is the larger architectural
//! lift the brief flagged as optional; staged as a future change to
//! avoid expanding the diff into every `capability_covers` match arm.

use argo_telemetry::prometheus_registry;

/// Cheap path-prefix check. Mirrors `routes_openmetrics::is_openmetrics_request`
/// so the accept loop can dispatch `/metrics/cardinality` outside of
/// the JSON response builder.
pub(super) fn is_metrics_cardinality_request(req: &str) -> bool {
    req.starts_with("GET /metrics/cardinality ")
        || req.starts_with("GET /metrics/cardinality?")
}

/// Build the full HTTP response bytes for `/metrics/cardinality`.
/// Returned as bytes so the caller can write straight into the
/// `TcpStream` without going through the JSON response builder.
#[must_use]
pub(super) fn http_response() -> Vec<u8> {
    let snapshot = prometheus_registry::cardinality_snapshot();
    let mut families = serde_json::Map::with_capacity(snapshot.len());
    for row in snapshot {
        families.insert(
            row.metric,
            serde_json::json!({
                "current_count": row.current_count,
                "cap": row.cap,
                "capped_total": row.capped_total,
            }),
        );
    }
    let body = serde_json::json!({ "families": families }).to_string();
    let resp = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    resp.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_prefix_check_matches_get_and_query() {
        assert!(is_metrics_cardinality_request(
            "GET /metrics/cardinality HTTP/1.1\r\n"
        ));
        assert!(is_metrics_cardinality_request(
            "GET /metrics/cardinality?foo=bar HTTP/1.1\r\n"
        ));
        assert!(!is_metrics_cardinality_request(
            "GET /metrics/cardinalityfoo HTTP/1.1\r\n"
        ));
        assert!(!is_metrics_cardinality_request(
            "POST /metrics/cardinality HTTP/1.1\r\n"
        ));
    }

    #[test]
    fn response_is_json_with_families_key() {
        let bytes = http_response();
        let s = std::str::from_utf8(&bytes).expect("response is utf-8");
        assert!(s.starts_with("HTTP/1.1 200 OK"));
        assert!(s.contains("Content-Type: application/json"));
        let body_start = s.find("\r\n\r\n").unwrap() + 4;
        let body = &s[body_start..];
        let parsed: serde_json::Value =
            serde_json::from_str(body).expect("body is valid json");
        assert!(parsed.get("families").is_some());
    }
}
