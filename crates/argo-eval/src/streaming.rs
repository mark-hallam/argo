//! Streaming output for `argo eval run` (Stream
//! `claude-eval-streaming`).
//!
//! Three modes:
//! - `Off` — batch (existing behaviour); the runner prints
//!   nothing per fixture and emits one summary at the end.
//! - `Simple` (default) — one human-readable line per fixture as
//!   it completes, e.g. `[3/12] verify_price: PASS`.
//! - `Verbose` — one NDJSON object per fixture for piping into
//!   `jq` or other downstream pipelines.
//!
//! The argo-eval crate exposes the format helpers here so both
//! the CLI handler and the vertical-fixture runner can share the
//! same wire shape.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Streaming output mode selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamMode {
    /// Batch — no per-fixture output, only the final summary.
    Off,
    /// One human-readable line per fixture (default).
    #[default]
    Simple,
    /// One NDJSON object per fixture, machine-readable.
    Verbose,
}

impl StreamMode {
    /// Parse `off` / `simple` / `verbose` (case-insensitive).
    /// Empty string → `Simple` (the default). Returns `None` for
    /// any other input so callers can surface a clear error.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "" | "simple" => Some(Self::Simple),
            "off" | "batch" => Some(Self::Off),
            "verbose" | "ndjson" => Some(Self::Verbose),
            _ => None,
        }
    }

    /// Whether per-fixture output should be emitted as it completes.
    #[must_use]
    pub fn streams_per_fixture(self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// Format a `Simple` per-fixture progress line per the brief:
/// `[<idx>/<total>] <id>: <PASS|FAIL>` plus optional duration +
/// cost. `idx` is 1-based.
#[must_use]
pub fn format_simple_line(
    idx: usize,
    total: usize,
    id: &str,
    passed: bool,
    duration_ms: Option<u64>,
    cost_usd: Option<f64>,
) -> String {
    let marker = if passed { "PASS" } else { "FAIL" };
    let mut suffix = String::new();
    match (duration_ms, cost_usd) {
        (Some(ms), Some(c)) => suffix.push_str(&format!(" ({ms}ms, ${c:.4})")),
        (Some(ms), None) => suffix.push_str(&format!(" ({ms}ms)")),
        (None, Some(c)) => suffix.push_str(&format!(" (${c:.4})")),
        (None, None) => {}
    }
    format!("[{idx}/{total}] {id}: {marker}{suffix}")
}

/// Format a `Verbose` NDJSON line. The `extra` payload is merged
/// into the envelope so callers can attach scoring / fixture
/// metadata without going through a struct.
#[must_use]
pub fn format_verbose_line(
    idx: usize,
    total: usize,
    id: &str,
    passed: bool,
    extra: Value,
) -> String {
    let mut envelope = serde_json::json!({
        "type": "fixture",
        "idx": idx,
        "total": total,
        "id": id,
        "passed": passed,
    });
    if let (Some(map), Some(other)) = (envelope.as_object_mut(), extra.as_object()) {
        for (k, v) in other {
            map.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    envelope.to_string()
}

/// Format the trailing summary line. In `Simple` mode it's a
/// single human-readable line; in `Verbose` mode it's an NDJSON
/// envelope with `type=summary`. `Off` callers should NOT call
/// this — they emit their own summary path.
#[must_use]
pub fn format_summary_line(
    mode: StreamMode,
    passed: usize,
    failed: usize,
    total_ms: Option<u64>,
) -> String {
    match mode {
        StreamMode::Off => String::new(),
        StreamMode::Simple => {
            let total = passed + failed;
            let suffix = total_ms
                .map(|ms| format!(" in {ms}ms"))
                .unwrap_or_default();
            format!("summary: {passed}/{total} passed{suffix}")
        }
        StreamMode::Verbose => serde_json::json!({
            "type": "summary",
            "passed": passed,
            "failed": failed,
            "total_ms": total_ms,
        })
        .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_accepts_known_modes() {
        assert_eq!(StreamMode::parse("off"), Some(StreamMode::Off));
        assert_eq!(StreamMode::parse("simple"), Some(StreamMode::Simple));
        assert_eq!(StreamMode::parse("verbose"), Some(StreamMode::Verbose));
        assert_eq!(StreamMode::parse("BATCH"), Some(StreamMode::Off));
        assert_eq!(StreamMode::parse("NDJSON"), Some(StreamMode::Verbose));
        assert_eq!(StreamMode::parse(""), Some(StreamMode::Simple));
        assert_eq!(StreamMode::parse("garbage"), None);
    }

    #[test]
    fn streams_per_fixture_only_when_not_off() {
        assert!(!StreamMode::Off.streams_per_fixture());
        assert!(StreamMode::Simple.streams_per_fixture());
        assert!(StreamMode::Verbose.streams_per_fixture());
    }

    #[test]
    fn format_simple_line_with_duration_and_cost() {
        let line = format_simple_line(3, 12, "verify_price", true, Some(172), Some(0.0021));
        assert_eq!(line, "[3/12] verify_price: PASS (172ms, $0.0021)");
    }

    #[test]
    fn format_simple_line_with_just_id() {
        let line = format_simple_line(1, 5, "first", false, None, None);
        assert_eq!(line, "[1/5] first: FAIL");
    }

    #[test]
    fn format_verbose_line_emits_valid_ndjson() {
        let line = format_verbose_line(
            2,
            5,
            "fixture_b",
            true,
            json!({"score": 0.92, "suite": "core"}),
        );
        let parsed: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed.get("type").and_then(Value::as_str), Some("fixture"));
        assert_eq!(parsed.get("idx").and_then(Value::as_u64), Some(2));
        assert_eq!(parsed.get("id").and_then(Value::as_str), Some("fixture_b"));
        assert_eq!(parsed.get("passed").and_then(Value::as_bool), Some(true));
        assert_eq!(parsed.get("score").and_then(Value::as_f64), Some(0.92));
    }

    #[test]
    fn format_verbose_line_does_not_overwrite_envelope_keys() {
        // If the caller's `extra` accidentally carries `id`,
        // `idx`, `passed`, etc., the envelope keys stay
        // authoritative.
        let line = format_verbose_line(
            1,
            1,
            "real-id",
            true,
            json!({"id": "spoof", "passed": false, "extra": "ok"}),
        );
        let parsed: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed.get("id").and_then(Value::as_str), Some("real-id"));
        assert_eq!(parsed.get("passed").and_then(Value::as_bool), Some(true));
        assert_eq!(parsed.get("extra").and_then(Value::as_str), Some("ok"));
    }

    #[test]
    fn format_summary_simple_includes_total_ms() {
        let s = format_summary_line(StreamMode::Simple, 4, 1, Some(1234));
        assert_eq!(s, "summary: 4/5 passed in 1234ms");
    }

    #[test]
    fn format_summary_verbose_emits_ndjson() {
        let s = format_summary_line(StreamMode::Verbose, 3, 2, Some(500));
        let parsed: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.get("type").and_then(Value::as_str), Some("summary"));
        assert_eq!(parsed.get("passed").and_then(Value::as_u64), Some(3));
        assert_eq!(parsed.get("failed").and_then(Value::as_u64), Some(2));
    }

    #[test]
    fn format_summary_off_returns_empty() {
        assert_eq!(format_summary_line(StreamMode::Off, 1, 0, Some(1)), "");
    }

    #[test]
    fn default_mode_is_simple() {
        assert_eq!(StreamMode::default(), StreamMode::Simple);
    }
}
