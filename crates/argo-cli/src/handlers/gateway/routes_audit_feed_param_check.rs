//! Strict-mode query-param check for the audit feed endpoint.
//!
//! The audit feed used to silently ignore unknown query parameters —
//! e.g. `?kind=worker_duty.fired` was dropped, the endpoint returned
//! every row, and the caller had no signal that the filter never ran.
//! That gap let the v1.0 demo runbook drift to `kind = …` while the
//! gateway only honored `action_type = …`, so a diligent operator
//! transcribing the runbook URL into a curl probe would see "all
//! rows" instead of the one row the narration promised.
//!
//! Two callbacks for the parent route handler:
//! - [`unknown_param`] surfaces the first unknown key so the handler
//!   can `400 unknown_query_param` instead of silently filtering away.
//! - [`action_types`] folds the canonical `?action_type=` and the new
//!   `?kind=` alias into one `Vec<String>` so both wire spellings hit
//!   the same filter path.

/// Every query parameter accepted by `GET /api/v1/audit`. `kind` is
/// the alias for `action_type` — see module docs.
const KNOWN_PARAMS: &[&str] = &[
    "limit",
    "range",
    "from",
    "to",
    "cursor",
    "q",
    "worker",
    "channel",
    "action_type",
    "kind",
    "status",
    "approver",
    "cost_band",
    "classification",
    "as_tenant",
];

/// Returns the first unknown query parameter name in `tail`, or `None`
/// if every key parses as one of [`KNOWN_PARAMS`]. `tail` is the raw
/// request line minus the HTTP verb (matching the parent module's
/// internal calling convention).
///
/// # Examples
///
/// ```ignore
/// // (private helper — illustration only)
/// // assert!(unknown_param("/api/v1/audit?limit=10 HTTP/1.1").is_none());
/// // assert_eq!(unknown_param("/api/v1/audit?bogus=x HTTP/1.1").as_deref(), Some("bogus"));
/// ```
pub(super) fn unknown_param(tail: &str) -> Option<String> {
    let head = tail.split_once(' ').map(|(p, _)| p).unwrap_or(tail);
    let qs = head.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let key = kv.split_once('=').map_or(kv, |(k, _)| k);
        (!key.is_empty() && !KNOWN_PARAMS.contains(&key)).then(|| key.to_string())
    })
}

/// Merges `?action_type=` and the `?kind=` alias into a single filter
/// list so the downstream filter pass sees both wire spellings as one
/// dimension. Empty input returns an empty `Vec`.
pub(super) fn action_types(tail: &str) -> Vec<String> {
    let mut v = super::csv_param(tail, "action_type");
    v.extend(super::csv_param(tail, "kind"));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_params_pass() {
        assert!(unknown_param("/api/v1/audit?action_type=foo&kind=bar HTTP/1.1").is_none());
        assert!(unknown_param("/api/v1/audit?limit=10&q=x HTTP/1.1").is_none());
        assert!(unknown_param("/api/v1/audit HTTP/1.1").is_none());
    }

    #[test]
    fn unknown_flagged() {
        assert_eq!(
            unknown_param("/api/v1/audit?bogus=x HTTP/1.1").as_deref(),
            Some("bogus"),
        );
    }

    #[test]
    fn action_types_merges_kind_alias() {
        let merged = action_types("/api/v1/audit?action_type=a&kind=b HTTP/1.1");
        assert_eq!(merged, vec!["a".to_string(), "b".to_string()]);
    }
}
