//! Audit-chain replay formatter (Stream `claude-audit-replay`).
//!
//! Two formats:
//! - `Pretty`: one-line headline per row — seq, kind, truncated
//!   payload — for skim-reading an incident timeline.
//! - `Raw`: full row as one NDJSON line per call — for piping
//!   into `jq` or downstream pipelines.
//!
//! Both formats share a filter pass (`from_seq` lower bound +
//! `until` upper bound on `created_at`).
use crate::export::ExportedRow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Convert micros (the `created_at_micros` column type) to a UTC
/// timestamp for the `--until` upper-bound check.
fn micros_to_utc(micros: i64) -> Option<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp_micros(micros)
}

/// Output format selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplayFormat {
    /// One-line headlines (default).
    Pretty,
    /// Full NDJSON dump.
    Raw,
}

impl ReplayFormat {
    /// Parse `pretty` / `raw` (case-insensitive).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pretty" => Some(Self::Pretty),
            "raw" => Some(Self::Raw),
            _ => None,
        }
    }
}

/// Filter the row list by `from_seq` lower bound + `until`
/// (RFC-3339 timestamp) upper bound. Both are optional.
#[must_use]
pub fn filter_rows(
    rows: Vec<ExportedRow>,
    from_seq: Option<i64>,
    until: Option<DateTime<Utc>>,
) -> Vec<ExportedRow> {
    rows.into_iter()
        .filter(|r| from_seq.is_none_or(|min| r.seq >= min))
        .filter(|r| {
            until.is_none_or(|max| {
                micros_to_utc(r.created_at_micros)
                    .map(|t| t <= max)
                    .unwrap_or(true)
            })
        })
        .collect()
}

/// Render every row to its formatted lines per the brief.
#[must_use]
pub fn render(rows: &[ExportedRow], format: ReplayFormat) -> String {
    let mut out = String::new();
    for row in rows {
        let line = match format {
            ReplayFormat::Pretty => render_pretty(row),
            ReplayFormat::Raw => render_raw(row),
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn render_pretty(row: &ExportedRow) -> String {
    let payload_short = truncate_payload(&row.payload, 80);
    let session_short = row
        .session_id
        .as_deref()
        .map(|s| {
            if s.len() >= 8 {
                s[..8].to_string()
            } else {
                s.to_string()
            }
        })
        .unwrap_or_else(|| "(no-session)".to_string());
    format!(
        "[{:>6}] {} {:<22} {}",
        row.seq, session_short, row.kind, payload_short
    )
}

fn render_raw(row: &ExportedRow) -> String {
    serde_json::to_string(row).unwrap_or_else(|_| "{}".to_string())
}

fn truncate_payload(payload: &serde_json::Value, max: usize) -> String {
    let canonical = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    if canonical.chars().count() <= max {
        canonical
    } else {
        let truncated: String = canonical.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(seq: i64, kind: &str, payload: serde_json::Value, ts: &str) -> ExportedRow {
        let micros = ts
            .parse::<DateTime<Utc>>()
            .map(|t| t.timestamp_micros())
            .unwrap_or(0);
        ExportedRow {
            seq,
            id: format!("00000000-0000-0000-0000-{seq:012}"),
            session_id: Some("01HXSESSION01000000000000000000".to_string()),
            vertical_id: None,
            kind: kind.to_string(),
            payload,
            parent_hash: "00".repeat(32),
            self_hash: "00".repeat(32),
            created_at_micros: micros,
        }
    }

    #[test]
    fn parse_format_accepts_both() {
        assert_eq!(ReplayFormat::parse("pretty"), Some(ReplayFormat::Pretty));
        assert_eq!(ReplayFormat::parse("RAW"), Some(ReplayFormat::Raw));
        assert_eq!(ReplayFormat::parse("xml"), None);
    }

    #[test]
    fn filter_rows_applies_from_seq() {
        let rows = vec![
            row(1, "turn_start", json!({}), "2026-01-01T00:00:00Z"),
            row(5, "turn_end", json!({}), "2026-01-01T00:00:01Z"),
            row(9, "turn_start", json!({}), "2026-01-01T00:00:02Z"),
        ];
        let out = filter_rows(rows, Some(5), None);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].seq, 5);
    }

    #[test]
    fn filter_rows_applies_until_timestamp() {
        let rows = vec![
            row(1, "turn_start", json!({}), "2026-01-01T00:00:00Z"),
            row(2, "turn_start", json!({}), "2026-01-02T00:00:00Z"),
            row(3, "turn_start", json!({}), "2026-01-03T00:00:00Z"),
        ];
        let until: DateTime<Utc> = "2026-01-02T12:00:00Z".parse().unwrap();
        let out = filter_rows(rows, None, Some(until));
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|r| r.seq <= 2));
    }

    #[test]
    fn pretty_truncates_long_payloads_to_80_chars() {
        let big = json!({"text": "x".repeat(200)});
        let r = row(42, "tool_call", big, "2026-01-01T00:00:00Z");
        let line = render_pretty(&r);
        // Layout: "[seq] sess kind payload" — strip everything
        // before the payload to count the payload portion.
        let payload_start = line
            .rfind("tool_call")
            .map(|i| i + "tool_call".len())
            .unwrap();
        let payload = line[payload_start..].trim();
        assert!(payload.chars().count() <= 80, "got {} chars", payload.chars().count());
        assert!(payload.ends_with('…'));
    }

    #[test]
    fn pretty_handles_short_payload() {
        let r = row(1, "turn_start", json!({}), "2026-01-01T00:00:00Z");
        let line = render_pretty(&r);
        assert!(line.contains("turn_start"));
        assert!(line.contains("[     1]"));
    }

    #[test]
    fn raw_emits_valid_ndjson() {
        let r = row(1, "tool_result", json!({"ok": true}), "2026-01-01T00:00:00Z");
        let out = render(&[r], ReplayFormat::Raw);
        let line = out.trim();
        let parsed: serde_json::Value = serde_json::from_str(line).expect("valid JSON");
        assert_eq!(parsed.get("seq").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(parsed.get("kind").and_then(|v| v.as_str()), Some("tool_result"));
    }

    #[test]
    fn render_emits_one_line_per_row() {
        let rows = vec![
            row(1, "turn_start", json!({}), "2026-01-01T00:00:00Z"),
            row(2, "turn_end", json!({}), "2026-01-01T00:00:01Z"),
        ];
        let out = render(&rows, ReplayFormat::Pretty);
        assert_eq!(out.lines().count(), 2);
    }

    #[test]
    fn render_in_seq_order() {
        // Caller is responsible for upstream ordering; render
        // emits exactly the order it received. Verify by passing
        // a deliberately out-of-order vec — render preserves it,
        // it doesn't re-sort.
        let rows = vec![
            row(7, "kind", json!({}), "2026-01-01T00:00:00Z"),
            row(3, "kind", json!({}), "2026-01-01T00:00:00Z"),
        ];
        let out = render(&rows, ReplayFormat::Pretty);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].contains("[     7]"));
        assert!(lines[1].contains("[     3]"));
    }
}
