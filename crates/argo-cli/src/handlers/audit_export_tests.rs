//! Handler-level fixture tests for `argo audit export`.
//!
//! Covers golden output for the new `mission-audit-csv-export`
//! formats (`csv`, `jsonl`, `json`) plus the `--kind` filter
//! predicate. The existing `tarball`/`ndjson` flow is regression-
//! protected by `crates/argo-cli/tests/audit_export_smoke.rs`; this
//! module focuses on per-format encoding decisions.
use super::*;
use crate::handlers::audit_export_args::NormalizedExport;
use std::cell::RefCell;

fn args(format: &str) -> ExportArgs {
    ExportArgs {
        format: format.into(),
        bundle_out: None,
        worker_id: None,
        tenant: None,
        session: None,
        since: None,
        until: None,
        since_minutes: None,
        gdpr_redact_emails: false,
        json: false,
        kinds: None,
    }
}

fn sample_row(seq: i64, kind: &str) -> ExportedRow {
    ExportedRow {
        seq,
        id: format!("row-{seq}"),
        session_id: Some("session-1".into()),
        vertical_id: None,
        kind: kind.into(),
        payload: serde_json::json!({
            "tool": "shell",
            "tenant_id": "ws-default",
            "worker_id": "worker-a",
            "email": "ops@example.com",
        }),
        parent_hash: "00".repeat(32),
        self_hash: "11".repeat(32),
        created_at_micros: 1_778_025_600_000_000,
    }
}

fn stream_rows(format: &str, rows: &[ExportedRow]) -> (Vec<u8>, ArtifactSummary) {
    let query = NormalizedExport::new(args(format)).unwrap();
    let format = query.format;
    let sink: Vec<u8> = Vec::new();
    let counter = RefCell::new(WriteAccount::default());
    let mut writer = HashingWriter::new(sink, &counter);
    write_prologue(&mut writer, format).unwrap();
    let mut first = true;
    for row in rows {
        emit_row(&mut writer, format, row, &mut first).unwrap();
    }
    write_epilogue(&mut writer, format).unwrap();
    writer.flush().unwrap();
    let buf = {
        let HashingWriter { inner, .. } = writer;
        inner
    };
    let summary = counter.into_inner().finalize();
    (buf, summary)
}

#[test]
fn csv_format_emits_brief_spec_columns() {
    let (bytes, summary) = stream_rows("csv", &[sample_row(1, "tool_call")]);
    let body = String::from_utf8(bytes).unwrap();
    assert!(body.starts_with("seq,id,session_id,"));
    assert!(body.contains(",tenant_id,worker_id,timestamp\r\n"));
    assert!(body.contains("ws-default,worker-a,2026-05-06T00:00:00+00:00\r\n"));
    assert!(summary.sha256.len() == 64);
    assert_eq!(summary.bytes, body.len());
}

#[test]
fn jsonl_format_emits_one_object_per_line() {
    let rows = [sample_row(1, "tool_call"), sample_row(2, "tool_result")];
    let (bytes, _) = stream_rows("jsonl", &rows);
    let body = String::from_utf8(bytes).unwrap();
    let lines: Vec<&str> = body.split('\n').filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(parsed["seq"].is_i64());
        assert!(parsed["kind"].is_string());
    }
}

#[test]
fn json_format_emits_single_top_level_array() {
    let rows = [sample_row(1, "tool_call"), sample_row(2, "tool_result")];
    let (bytes, _) = stream_rows("json", &rows);
    let body = String::from_utf8(bytes).unwrap();
    assert!(body.starts_with('['));
    assert!(body.ends_with(']'));
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = parsed.as_array().expect("top-level array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["kind"], "tool_call");
    assert_eq!(arr[1]["kind"], "tool_result");
}

#[test]
fn json_format_with_no_rows_emits_empty_array() {
    let (bytes, _) = stream_rows("json", &[]);
    let body = String::from_utf8(bytes).unwrap();
    assert_eq!(body, "[]");
}

#[test]
fn kind_filter_matches_only_listed_labels() {
    let mut a = args("csv");
    a.kinds = Some("tool_call, halt".into());
    let query = NormalizedExport::new(a).unwrap();
    assert!(query.matches(&sample_row(1, "tool_call")));
    assert!(!query.matches(&sample_row(2, "tool_result")));
    assert!(query.matches(&sample_row(3, "halt")));
}

#[test]
fn duration_since_filter_admits_recent_rows() {
    let mut a = args("csv");
    a.since = Some("1h".into());
    let query = NormalizedExport::new(a).unwrap();
    let now = chrono::Utc::now().timestamp_micros();
    let mut row = sample_row(1, "tool_call");
    row.created_at_micros = now - 60_000_000; // 1 minute ago — inside window
    assert!(query.matches(&row));
    row.created_at_micros = now - 2 * 3_600_000_000_i64; // 2 hours ago — outside
    assert!(!query.matches(&row));
}

#[test]
fn csv_payload_with_commas_quotes_and_newlines_roundtrips() {
    // Build a payload whose canonical-form JSON, after CSV quoting,
    // exercises every RFC 4180 trouble character: `,`, `"`, `\r`,
    // and `\n` MUST all appear inside the quoted field. The whole
    // file must split cleanly into the header row + one record row
    // when parsed by the in-test RFC 4180 reader below.
    let mut row = sample_row(1, "tool_call");
    row.payload = serde_json::json!({
        "tenant_id": "ws-default",
        "worker_id": "worker-a",
        // String value with embedded comma, quote, CR, LF. After
        // canonicalization and CSV quoting the field should round-
        // trip cleanly without splitting the record on the bare LF.
        "note": "comma, quote\", lf\nand cr\r",
    });
    let (bytes, _) = stream_rows("csv", &[row]);
    let records = parse_rfc4180(&bytes);
    assert_eq!(records.len(), 2, "expected header + 1 row, got {records:?}");
    assert_eq!(records[0][0], "seq");
    assert_eq!(records[1][0], "1");
    // The payload column is JSON-encoded, so embedded `"` shows up
    // as `\"` in the decoded field text. Parse it back to JSON and
    // check the original value survived.
    let payload_idx = 6;
    let payload: serde_json::Value = serde_json::from_str(&records[1][payload_idx])
        .expect("payload column must be valid canonical JSON");
    assert_eq!(payload["note"], "comma, quote\", lf\nand cr\r");
    assert_eq!(payload["tenant_id"], "ws-default");
    assert_eq!(payload["worker_id"], "worker-a");
}

/// Tiny RFC 4180 parser used by the round-trip test. Returns a
/// `Vec<Vec<String>>` of fields per logical record.
fn parse_rfc4180(input: &[u8]) -> Vec<Vec<String>> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut field = String::new();
    let mut record: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut i = 0;
    while i < input.len() {
        let c = input[i] as char;
        if in_quotes {
            if c == '"' {
                if i + 1 < input.len() && input[i + 1] == b'"' {
                    field.push('"');
                    i += 2;
                    continue;
                }
                in_quotes = false;
                i += 1;
                continue;
            }
            field.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_quotes = true;
            i += 1;
            continue;
        }
        if c == ',' {
            record.push(std::mem::take(&mut field));
            i += 1;
            continue;
        }
        if c == '\r' && i + 1 < input.len() && input[i + 1] == b'\n' {
            record.push(std::mem::take(&mut field));
            records.push(std::mem::take(&mut record));
            i += 2;
            continue;
        }
        field.push(c);
        i += 1;
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}
