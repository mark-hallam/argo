//! Rendering and display helpers for `argo activity since`.
//!
//! Pure presentation layer extracted from `activity_since.rs`: NDJSON
//! and text serialization, plus the small time/string formatting
//! utilities those routines (and `row_to_activity`) depend on.

use super::activity_since::ActivityRow;
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
struct ActivityEnvelope<'a> {
    r#type: &'static str,
    activity: &'a ActivityRow,
}

pub(crate) fn print_ndjson(rows: &[ActivityRow]) -> ArgoResult<()> {
    for row in rows {
        let line = serde_json::to_string(&ActivityEnvelope {
            r#type: "activity",
            activity: row,
        })
        .map_err(|e| ArgoError::Provider(format!("serialize activity: {e}")))?;
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn print_text_rows(rows: &[ActivityRow]) {
    if rows.is_empty() {
        println!("(no activity matched)");
        return;
    }
    for row in rows {
        println!("{}", format_text_row(row));
    }
}

pub(crate) fn format_text_row(row: &ActivityRow) -> String {
    format!(
        "{} [{}] [{}] [{}] {}",
        format_hms(row.created_at_micros),
        row.worker_short_id,
        row.action_kind,
        row.decision.as_deref().unwrap_or("-"),
        row.message
    )
}

pub(crate) fn short_id(raw: &str) -> String {
    if raw.len() <= 12 {
        raw.to_string()
    } else {
        raw.chars().take(8).collect()
    }
}

pub(crate) fn uuid_or_hex(bytes: &[u8]) -> String {
    Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| hex::encode(bytes))
}

pub(crate) fn format_rfc3339(micros: i64) -> String {
    let Some(dt) = datetime(micros) else {
        return micros.to_string();
    };
    dt.to_rfc3339()
}

fn format_hms(micros: i64) -> String {
    format_dt(micros, "%H:%M:%S").unwrap_or_else(|| "--:--:--".into())
}

fn format_dt(micros: i64, fmt: &str) -> Option<String> {
    datetime(micros).map(|dt| dt.format(fmt).to_string())
}

fn datetime(micros: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    let secs = micros.div_euclid(1_000_000);
    let nanos = u32::try_from(micros.rem_euclid(1_000_000) * 1_000).ok()?;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_activity_line_from_payload_fields() {
        let row = ActivityRow {
            seq: 1,
            id: "row".into(),
            session_id: None,
            tenant_id: None,
            vertical_id: None,
            audit_kind: "tool_call".into(),
            action_kind: "send_email".into(),
            decision: Some("ask".into()),
            worker_id: Some("worker-abcdefghijk".into()),
            worker_short_id: "worker-a".into(),
            message: "draft approval".into(),
            created_at: "1970-01-01T00:00:00+00:00".into(),
            created_at_micros: 0,
            payload: serde_json::json!({}),
        };
        assert_eq!(
            format_text_row(&row),
            "00:00:00 [worker-a] [send_email] [ask] draft approval"
        );
    }
}
