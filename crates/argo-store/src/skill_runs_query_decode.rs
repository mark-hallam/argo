//! Decoding and filtering helpers for [`SkillRunRow`].
//!
//! Extracted from `skill_runs_query.rs` to keep that file under the 500-LOC
//! invariant. The functions here are pure (no I/O) and crate-private.

use crate::{SkillRunRow, StoreResult};
use argo_types::ArgoError;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

/// Decode a `worker_action` memory body into a [`SkillRunRow`].
///
/// Returns `Ok(None)` when the row is missing the action id or skill name.
pub(crate) fn parse_run(body: &str, learned_at: i64) -> StoreResult<Option<SkillRunRow>> {
    let value: Value = serde_json::from_str(body)
        .map_err(|e| ArgoError::Provider(format!("worker action JSON: {e}")))?;
    let Some(action_id) = string_at(&value, &["action_id"]) else {
        return Ok(None);
    };
    let Some(skill_name) = skill_name(&value) else {
        return Ok(None);
    };
    let worker_id = worker_id(&value);
    let tenant_id = string_at(&value, &["payload", "tenant_id"]);
    let created_at = string_at(&value, &["created_at"]);
    let (timestamp_micros, timestamp) = timestamp_pair(created_at.as_deref(), learned_at);
    let state = string_at(&value, &["state"]).unwrap_or_else(|| "skipped".to_string());
    let decision = decision_for_state(&state).to_string();
    let latency_ms = explicit_latency(&value).or_else(|| derived_latency(&value));
    Ok(Some(SkillRunRow {
        skill_name,
        timestamp,
        timestamp_micros,
        worker_id,
        tenant_id,
        decision,
        latency_ms,
        action_id,
        spend_usd_micros: 0,
    }))
}

/// Filter, sort newest-first, and truncate a batch of [`SkillRunRow`] values.
pub(crate) fn filter_runs(
    mut rows: Vec<SkillRunRow>,
    skill_name: Option<&str>,
    tenant_id: Option<&str>,
    worker_id: Option<&str>,
    since_micros: Option<i64>,
    limit: usize,
    include_failed: bool,
) -> Vec<SkillRunRow> {
    rows.retain(|row| {
        matches_opt(Some(row.skill_name.as_str()), skill_name)
            && matches_opt(row.tenant_id.as_deref(), tenant_id)
            && matches_opt(Some(row.worker_id.as_str()), worker_id)
            && match since_micros {
                Some(cutoff) => row.timestamp_micros >= cutoff,
                None => true,
            }
            && (include_failed || row.decision == "success")
    });
    rows.sort_by(|a, b| {
        b.timestamp_micros
            .cmp(&a.timestamp_micros)
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
    rows.truncate(limit);
    rows
}

/// Tie-break helper used when a duplicate action id appears across rows.
pub(crate) fn newer_or_equal(prev: &SkillRunRow, next: &SkillRunRow) -> bool {
    prev.timestamp_micros > next.timestamp_micros
        || (prev.timestamp_micros == next.timestamp_micros && prev.action_id <= next.action_id)
}

fn matches_opt(actual: Option<&str>, want: Option<&str>) -> bool {
    match want {
        Some(want) => actual == Some(want),
        None => true,
    }
}

fn skill_name(value: &Value) -> Option<String> {
    string_at(value, &["payload", "skill_used"])
        .or_else(|| string_at(value, &["payload", "skill"]))
        .or_else(|| string_at(value, &["payload", "skill_name"]))
}

fn worker_id(value: &Value) -> String {
    string_at(value, &["payload", "worker_id"])
        .or_else(|| string_at(value, &["worker_id"]))
        .unwrap_or_else(|| "-".to_string())
}

fn decision_for_state(state: &str) -> &'static str {
    match state {
        "completed" => "success",
        "failed" => "fail",
        _ => "skipped",
    }
}

fn explicit_latency(value: &Value) -> Option<i64> {
    int_at(value, &["payload", "latency_ms"])
        .or_else(|| int_at(value, &["payload", "duration_ms"]))
        .or_else(|| int_at(value, &["result", "latency_ms"]))
        .or_else(|| int_at(value, &["result", "duration_ms"]))
}

fn derived_latency(value: &Value) -> Option<i64> {
    let created = string_at(value, &["created_at"]).and_then(|s| parse_time(&s));
    let executed = string_at(value, &["executed_at"]).and_then(|s| parse_time(&s));
    match (created, executed) {
        (Some(start), Some(end)) => Some((end - start).num_milliseconds().max(0)),
        _ => None,
    }
}

fn timestamp_pair(raw: Option<&str>, learned_at: i64) -> (i64, String) {
    if let Some(parsed) = raw.and_then(parse_time) {
        return (parsed.timestamp_micros(), parsed.to_rfc3339());
    }
    match Utc.timestamp_micros(learned_at).single() {
        Some(ts) => (learned_at, ts.to_rfc3339()),
        None => (learned_at, learned_at.to_string()),
    }
}

fn parse_time(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key)?;
    }
    match cur {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn int_at(value: &Value, path: &[&str]) -> Option<i64> {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key)?;
    }
    match cur {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().and_then(|v| i64::try_from(v).ok())),
        _ => None,
    }
}
