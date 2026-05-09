use super::*;
use chrono::TimeZone;

fn args() -> ApprovalHistoryArgs {
    ApprovalHistoryArgs {
        since: None,
        until: None,
        decision: Vec::new(),
        worker: None,
        action_kind: None,
        limit: 50,
        format: "table".into(),
    }
}

fn ws() -> WorkspaceId {
    WorkspaceId(uuid::Uuid::nil())
}

#[test]
fn relative_and_rfc3339_timestamps_are_supported() {
    let now = Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap();
    let mut a = args();
    a.since = Some("7d".into());
    let (_sql, params) = build_query_at(&a, ws(), now).unwrap();
    let expected = (now - chrono::Duration::days(7)).to_rfc3339();
    assert!(matches!(params[1], Param::Text(ref v) if v == &expected));

    let mut a = args();
    a.until = Some("2026-05-08T00:00:00Z".into());
    let (_sql, params) = build_query_at(&a, ws(), now).unwrap();
    assert!(matches!(params[1], Param::Text(ref v) if v.starts_with("2026-05-08T00:00:00")));
}

#[test]
fn decision_filters_emit_or_clause() {
    let mut a = args();
    a.decision = vec!["deny".into(), "timeout".into()];
    let (sql, params) = build_query_at(&a, ws(), Utc::now()).unwrap();
    assert!(sql.contains("decision IN (?,?)"));
    assert!(matches!(params[1], Param::Text(ref v) if v == "deny"));
    assert!(matches!(params[2], Param::Text(ref v) if v == "timeout"));
}

#[test]
fn worker_filter_is_parameterised() {
    let mut a = args();
    a.worker = Some("'; DROP TABLE memory_entry; --".into());
    let (sql, params) = build_query_at(&a, ws(), Utc::now()).unwrap();
    assert!(!sql.contains("DROP TABLE"));
    assert!(params
        .iter()
        .any(|p| matches!(p, Param::Text(v) if v.contains("DROP TABLE"))));
}

#[test]
fn limit_is_clamped_to_thousand() {
    let mut a = args();
    a.limit = 1001;
    let (_sql, params) = build_query_at(&a, ws(), Utc::now()).unwrap();
    assert!(matches!(params.last(), Some(Param::I64(1000))));
}

#[test]
fn zero_limit_falls_back_to_default() {
    let mut a = args();
    a.limit = 0;
    let (_sql, params) = build_query_at(&a, ws(), Utc::now()).unwrap();
    assert!(matches!(params.last(), Some(Param::I64(50))));
}

#[test]
fn ndjson_emits_one_object_per_line() {
    let rows = vec![
        ApprovalRow {
            id: "act-1".into(),
            requested_at: "2026-05-08T00:00:00+00:00".into(),
            decided_at: None,
            decision: "pending".into(),
            worker: "foo".into(),
            action_kind: "post_listing".into(),
            decided_by: None,
            latency_secs: None,
        },
        ApprovalRow {
            id: "act-2".into(),
            requested_at: "2026-05-08T00:01:00+00:00".into(),
            decided_at: Some("2026-05-08T00:02:00+00:00".into()),
            decision: "approve".into(),
            worker: "bar".into(),
            action_kind: "post_listing".into(),
            decided_by: Some("op".into()),
            latency_secs: Some(60),
        },
    ];
    let out = render(&rows, OutputFormat::Json);
    assert_eq!(out.lines().count(), 2);
    for line in out.lines() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(v.get("id").is_some());
    }
}

#[test]
fn markdown_renders_table_header() {
    let rows = vec![ApprovalRow {
        id: "act-1".into(),
        requested_at: "2026-05-08T00:00:00+00:00".into(),
        decided_at: Some("2026-05-08T00:01:00+00:00".into()),
        decision: "approve".into(),
        worker: "foo".into(),
        action_kind: "post_listing".into(),
        decided_by: Some("op".into()),
        latency_secs: Some(60),
    }];
    let out = render(&rows, OutputFormat::Markdown);
    assert!(out.starts_with("| id |"));
    assert!(out.contains("requested_at"));
    assert!(out.contains("latency_secs"));
    assert!(out.contains("| --- |"));
    assert!(out.contains("act-1"));
}

#[test]
fn empty_rows_render_a_friendly_message() {
    let out = render(&[], OutputFormat::Table);
    assert!(out.contains("no approval history rows"));
}

#[test]
fn unknown_format_is_rejected() {
    let err = OutputFormat::parse("yaml").unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("unknown format 'yaml'"));
}
