use super::*;
use argo_types::WorkspaceId;
use chrono::TimeZone;

fn args() -> ApprovalExportArgs {
    ApprovalExportArgs {
        since: None,
        until: None,
        decision: Vec::new(),
        worker: None,
        action_kind: None,
        limit: 50,
        format: "csv".into(),
        out: "-".into(),
        force: false,
    }
}

fn row() -> ApprovalExportRow {
    ApprovalExportRow {
        id: "act-1".into(),
        requested_at: "2026-01-01T00:00:00+00:00".into(),
        decided_at: Some("2026-01-01T00:02:00+00:00".into()),
        decision: "approve".into(),
        worker: "bot".into(),
        action_kind: "refund".into(),
        decided_by: Some("op".into()),
        latency_secs: Some(120),
        payload_excerpt: "hello, \"csv\"\nline".replace('\n', "\\n"),
    }
}

#[test]
fn build_query_uses_or_decisions_and_params() {
    let mut args = args();
    args.since = Some("7d".into());
    args.decision = vec!["deny".into(), "timeout".into()];
    args.worker = Some("'; DROP TABLE memory_entry; --".into());
    let now = Utc.with_ymd_and_hms(2026, 1, 8, 0, 0, 0).unwrap();
    let (sql, params) = build_query_at(&args, WorkspaceId::new(), now).unwrap();
    assert!(sql.contains("decision IN (?,?)"));
    assert!(sql.contains("worker = ?"));
    assert!(sql.contains("ORDER BY requested_at ASC, id ASC"));
    assert!(!sql.contains("DROP TABLE"));
    assert!(params.iter().any(|p| {
        matches!(p, Param::Text(v) if v == "'; DROP TABLE memory_entry; --")
    }));
}

#[test]
fn render_csv_has_header_and_escapes_payload() {
    let body = render(&[row()], ExportFormat::Csv).unwrap();
    let mut rdr = csv::Reader::from_reader(body.as_slice());
    assert_eq!(
        rdr.headers().unwrap(),
        [
            "id",
            "requested_at",
            "decided_at",
            "decision",
            "worker",
            "action_kind",
            "decided_by",
            "latency_secs",
            "payload_excerpt"
        ]
        .as_slice()
    );
    let records: Vec<_> = rdr.records().collect::<Result<_, _>>().unwrap();
    assert_eq!(records[0].get(8), Some("hello, \"csv\"\\nline"));
}

#[test]
fn render_json_and_jsonl_shapes() {
    let rows = vec![row()];
    let json: serde_json::Value =
        serde_json::from_slice(&render(&rows, ExportFormat::Json).unwrap()).unwrap();
    assert!(json.is_array());
    assert_eq!(json[0]["id"], "act-1");
    let jsonl = String::from_utf8(render(&rows, ExportFormat::Jsonl).unwrap()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(jsonl.trim()).unwrap();
    assert_eq!(parsed["payload_excerpt"], "hello, \"csv\"\\nline");
}

#[test]
fn payload_excerpt_truncates_and_escapes_newlines() {
    let raw = format!("{}{}", "a".repeat(80), "\nzzz");
    assert_eq!(payload_excerpt(Some(&raw)), format!("{}...", "a".repeat(80)));
    assert_eq!(payload_excerpt(Some("a\nb")), "a\\nb");
}

#[test]
fn format_and_decision_validation_are_strict() {
    assert_eq!(ExportFormat::parse("csv").unwrap(), ExportFormat::Csv);
    assert!(ExportFormat::parse("table").is_err());
    assert_eq!(parse_decision("pending").unwrap(), "pending");
    assert!(parse_decision("maybe").is_err());
}

#[test]
fn relative_dates_and_limit_clamp() {
    let now = Utc.with_ymd_and_hms(2026, 1, 8, 0, 0, 0).unwrap();
    assert_eq!(
        parse_relative_or_rfc3339("7d", now).unwrap(),
        "2026-01-01T00:00:00+00:00"
    );
    assert_eq!(clamped_limit(0), DEFAULT_LIMIT);
    assert_eq!(clamped_limit(1001), MAX_LIMIT);
}
