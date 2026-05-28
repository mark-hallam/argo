//! Verb / action_type strings that identify system-meta audit rows
//! (SPA self-pings, periodic KPI roll-ups, audit-page click telemetry).
//!
//! User-facing audit feeds (`GET /api/v1/audit` and the per-worker
//! `GET /api/v1/workers/<id>/activity`) hide these rows by default so
//! the camera reads agent turns, not the SPA's own audit trail. Both
//! endpoints honor `?include_meta=1` to opt back in for forensics.
//!
//! Two flavors in one list:
//!
//! - **`AuditKind` labels** (`http.request`, `worker.kpi_rolled`) —
//!   match the row's `kind` column and the resolved `verb` field that
//!   the activity-row JSON falls back to when `payload.verb` is unset.
//! - **`payload.endpoint` strings** (`audit.list.viewed`,
//!   `audit.detail.viewed`, `audit.replay`, `worker.activity.viewed`) —
//!   identify specific gateway routes whose self-view audit rows
//!   become noise in the user-facing feed.
//!
//! The filter checks every dimension (`kind`, `payload.verb`,
//! `payload.endpoint`, `payload.action_type`) so a row gets dropped
//! whether it carries the system label at the kind column or only at
//! the endpoint dimension.

/// Verb-like strings that mark a row as system-internal metadata
/// rather than an agent turn. See module doc for which dimensions to
/// match against.
pub const METADATA_VERBS: &[&str] = &[
    "http.request",
    "worker.kpi_rolled",
    "audit.list.viewed",
    "audit.detail.viewed",
    "audit.replay",
    "worker.activity.viewed",
];

/// `true` if `s` is one of [`METADATA_VERBS`].
#[must_use]
pub fn is_metadata_verb(s: &str) -> bool {
    METADATA_VERBS.contains(&s)
}

#[cfg(test)]
mod tests {
    use super::{is_metadata_verb, METADATA_VERBS};

    #[test]
    fn includes_core_self_traffic_labels() {
        assert!(is_metadata_verb("http.request"));
        assert!(is_metadata_verb("worker.kpi_rolled"));
        assert!(is_metadata_verb("audit.list.viewed"));
        assert!(is_metadata_verb("audit.detail.viewed"));
        assert!(is_metadata_verb("audit.replay"));
        assert!(is_metadata_verb("worker.activity.viewed"));
    }

    #[test]
    fn rejects_agent_turn_verbs() {
        assert!(!is_metadata_verb("message.sent"));
        assert!(!is_metadata_verb("approval.granted"));
        assert!(!is_metadata_verb("worker.hired"));
        assert!(!is_metadata_verb(""));
    }

    #[test]
    fn list_is_non_empty_and_unique() {
        assert!(!METADATA_VERBS.is_empty());
        let mut seen = METADATA_VERBS.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), METADATA_VERBS.len(), "duplicates in METADATA_VERBS");
    }
}
