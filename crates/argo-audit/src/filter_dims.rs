//! Denormalized filter-dimension extraction for `audit_record`
//! (Stream `mission-enterprise-audit-full-text-search`, raw slot 77).
//!
//! Migration v67 promotes the 7 J14 audit-feed filter dimensions
//! (worker / channel / action_type / outcome_status / approver /
//! cost_band / data_classification) to first-class nullable columns
//! on `audit_record`. The write path (`AuditSink::write`) calls
//! [`extract_filter_dims`] over the canonical JSON payload that's
//! about to be hashed and stored. The verify-chain path
//! (`verify::verify_session`) calls the same function over the
//! `payload_json` it reads back to assert the column and the JSON
//! never drift — the denorm sanity belt.
//!
//! Source of truth stays `payload_json`. The columns are an indexed
//! view; if a future column-vs-JSON drift is observed, the JSON
//! wins and the verifier flags the row.
//!
//! `cost_band` is the only derived dim: `cost_usd: f64` from the
//! payload is bucketed into the four-label band the frontend E5-b
//! filter UI uses. Missing or non-numeric `cost_usd` lands `None` —
//! the column distinguishes "no cost reported" from "cost = $0".

/// Denormalized filter-dimension values extracted from one audit
/// payload. Every field is `None` when the dim isn't present in
/// the JSON — historical rows (pre-v67 migration) read back all
/// `None`s, and the read path's `(col IN (?) OR col IS NULL)` SQL
/// gates the app-layer fallback for those.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilterDims {
    /// `payload_json.worker_id`.
    pub worker_id: Option<String>,
    /// `payload_json.channel`.
    pub channel: Option<String>,
    /// `payload_json.action_type`.
    pub action_type: Option<String>,
    /// `payload_json.outcome_status`, falling back to `.status`.
    pub outcome_status: Option<String>,
    /// `payload_json.approver_id`.
    pub approver_id: Option<String>,
    /// Bucketed from `payload_json.cost_usd` via
    /// [`cost_band_label`]; `None` when no cost is reported.
    pub cost_band: Option<String>,
    /// `payload_json.data_classification`, falling back to
    /// `.classification`.
    pub data_classification: Option<String>,
}

/// Cost-band labels matching the frontend E5-b filter UI:
/// `free` (≤ $0), `lt001` (<$0.01), `from001to10` (≥$0.01, <$10),
/// `gt10` (≥$10). Caller passes `None` for "no cost reported".
#[must_use]
pub fn cost_band_label(cost: f64) -> &'static str {
    if cost <= 0.0 {
        "free"
    } else if cost < 0.01 {
        "lt001"
    } else if cost < 10.0 {
        "from001to10"
    } else {
        "gt10"
    }
}

/// Extract the 7 filter dimensions from a parsed JSON payload.
///
/// Field-name resolution mirrors the J14 read-path
/// (`routes_audit_feed.rs::row_passes_filters`) so that the same
/// dimensional value lands in the column AND in the payload-pass
/// fallback. `cost_usd` → `cost_band` uses [`cost_band_label`].
///
/// Pure function — no allocations beyond the output strings, no
/// I/O. Cheap enough to call on every write.
#[must_use]
pub fn extract_filter_dims(value: &serde_json::Value) -> FilterDims {
    FilterDims {
        worker_id: payload_string(value, "worker_id"),
        channel: payload_string(value, "channel"),
        action_type: payload_string(value, "action_type"),
        outcome_status: payload_string(value, "outcome_status")
            .or_else(|| payload_string(value, "status")),
        approver_id: payload_string(value, "approver_id"),
        cost_band: payload_cost_usd(value).map(|c| cost_band_label(c).to_string()),
        data_classification: payload_string(value, "data_classification")
            .or_else(|| payload_string(value, "classification")),
    }
}

/// Re-extract dim values from a stored `payload_json` string. Used
/// by `verify::verify_session` to assert no drift between the
/// column and the JSON.
#[must_use]
pub fn extract_from_payload_json(payload_json: &str) -> Option<FilterDims> {
    let value: serde_json::Value = serde_json::from_str(payload_json).ok()?;
    Some(extract_filter_dims(&value))
}

fn payload_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| (!s.is_empty()).then(|| s.to_string()))
}

fn payload_cost_usd(value: &serde_json::Value) -> Option<f64> {
    value.get("cost_usd").and_then(serde_json::Value::as_f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_every_dimension_when_present() {
        let v = json!({
            "worker_id": "w1",
            "channel": "slack",
            "action_type": "approval.decided",
            "outcome_status": "succeeded",
            "approver_id": "a1",
            "cost_usd": 0.5,
            "data_classification": "internal",
        });
        let d = extract_filter_dims(&v);
        assert_eq!(d.worker_id.as_deref(), Some("w1"));
        assert_eq!(d.channel.as_deref(), Some("slack"));
        assert_eq!(d.action_type.as_deref(), Some("approval.decided"));
        assert_eq!(d.outcome_status.as_deref(), Some("succeeded"));
        assert_eq!(d.approver_id.as_deref(), Some("a1"));
        assert_eq!(d.cost_band.as_deref(), Some("from001to10"));
        assert_eq!(d.data_classification.as_deref(), Some("internal"));
    }

    #[test]
    fn missing_keys_land_none() {
        let v = json!({});
        let d = extract_filter_dims(&v);
        assert_eq!(d, FilterDims::default());
    }

    #[test]
    fn outcome_status_falls_back_to_status() {
        let v = json!({"status": "failed"});
        let d = extract_filter_dims(&v);
        assert_eq!(d.outcome_status.as_deref(), Some("failed"));
    }

    #[test]
    fn classification_falls_back_to_short_form() {
        let v = json!({"classification": "secret"});
        let d = extract_filter_dims(&v);
        assert_eq!(d.data_classification.as_deref(), Some("secret"));
    }

    #[test]
    fn cost_band_buckets_thresholds() {
        assert_eq!(cost_band_label(-1.0), "free");
        assert_eq!(cost_band_label(0.0), "free");
        assert_eq!(cost_band_label(0.0001), "lt001");
        assert_eq!(cost_band_label(0.009), "lt001");
        assert_eq!(cost_band_label(0.01), "from001to10");
        assert_eq!(cost_band_label(5.0), "from001to10");
        assert_eq!(cost_band_label(10.0), "gt10");
        assert_eq!(cost_band_label(1_000.0), "gt10");
    }

    #[test]
    fn cost_band_is_none_when_cost_missing() {
        let v = json!({"worker_id": "w1"});
        let d = extract_filter_dims(&v);
        assert_eq!(d.cost_band, None);
    }

    #[test]
    fn empty_string_values_land_none() {
        let v = json!({"worker_id": "", "channel": "slack"});
        let d = extract_filter_dims(&v);
        assert_eq!(d.worker_id, None);
        assert_eq!(d.channel.as_deref(), Some("slack"));
    }

    #[test]
    fn extract_from_payload_json_parses_then_extracts() {
        let s = r#"{"worker_id":"w1","cost_usd":50.0}"#;
        let d = extract_from_payload_json(s).expect("parsed");
        assert_eq!(d.worker_id.as_deref(), Some("w1"));
        assert_eq!(d.cost_band.as_deref(), Some("gt10"));
    }

    #[test]
    fn extract_from_payload_json_returns_none_on_bad_json() {
        assert!(extract_from_payload_json("not json").is_none());
    }
}
