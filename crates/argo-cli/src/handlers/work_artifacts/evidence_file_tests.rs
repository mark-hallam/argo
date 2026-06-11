//! File-evidence tests for the research-evidence gate
//! (`cu1e-evidence-gate-accept-file-evidence-for-fs-scoped-workers`).
//!
//! Pins: an fs-scoped worker grounded in a real `file_read` passes the
//! gate with zero web calls; a worker with zero evidence of either kind
//! stays blocked; errored reads don't count; attribution only credits
//! results whose call id maps to a `file_read` tool_use block.

use super::*;

/// An assistant row issuing one `file_read` tool_use with `id`.
fn assistant_file_read_use(id: &str) -> String {
    serde_json::json!([{
        "type": "tool_use",
        "id": id,
        "name": "file_read",
        "input": {"path": "Inbox/invoice-acme-4471.txt"},
    }])
    .to_string()
}

/// A tool row carrying one tool_result for call `id` with `text`.
fn tool_result(id: &str, text: &str, is_error: bool) -> String {
    serde_json::json!([{
        "type": "tool_result",
        "tool_use_id": id,
        "is_error": is_error,
        "content": [{"type": "text", "text": text}],
    }])
    .to_string()
}

const INVOICE: &str = "INVOICE #4471\nGlobex Corporation\nAmount due: $12,400.00\n\
                       Due date: 2026-06-01\nContact: accounts@globex.example\n\
                       Terms: net 30. Late fee applies after the due date.";

#[test]
fn fs_only_worker_with_file_read_evidence_is_sufficient() {
    let use_row = assistant_file_read_use("fr-1");
    let result_row = tool_result("fr-1", INVOICE, false);
    let rows: Vec<(&str, &str)> = vec![
        ("assistant", use_row.as_str()),
        ("tool", result_row.as_str()),
    ];
    let s = summarize_evidence(rows);
    assert_eq!(s.file_read_calls, 1);
    assert!(s.file_read_total_chars >= 100, "got {}", s.file_read_total_chars);
    assert_eq!(s.web_search_calls, 0);
    assert!(s.is_sufficient(), "file evidence alone must satisfy the gate");
    assert!(s.insufficiency_reason().is_empty());
}

#[test]
fn zero_evidence_of_either_kind_stays_insufficient() {
    let rows: Vec<(&str, &str)> =
        vec![("user", r#"[{"type":"text","text":"chase the invoice"}]"#)];
    let s = summarize_evidence(rows);
    assert!(!s.is_sufficient(), "zero grounding must stay blocked");
    let reason = s.insufficiency_reason();
    assert!(reason.contains("no web_search calls"), "got: {reason}");
    assert!(reason.contains("no file_read calls"), "got: {reason}");
}

#[test]
fn short_or_errored_file_reads_do_not_satisfy_the_gate() {
    // A 5-char read is below the 100-char floor.
    let use_row = assistant_file_read_use("fr-1");
    let short = tool_result("fr-1", "stub.", false);
    let s = summarize_evidence(vec![
        ("assistant", use_row.as_str()),
        ("tool", short.as_str()),
    ]);
    assert_eq!(s.file_read_calls, 1);
    assert!(!s.is_sufficient(), "a 5-char read is not grounding");
    assert!(
        s.insufficiency_reason().contains("file_read call(s) yielded only"),
        "got: {}",
        s.insufficiency_reason()
    );

    // An errored read counts as failed, not as evidence.
    let use_row = assistant_file_read_use("fr-2");
    let err = tool_result("fr-2", "failed to read Inbox/missing.txt: not found", true);
    let s = summarize_evidence(vec![
        ("assistant", use_row.as_str()),
        ("tool", err.as_str()),
    ]);
    assert_eq!(s.file_read_calls, 0);
    assert_eq!(s.file_read_failed, 1);
    assert!(!s.is_sufficient());
}

#[test]
fn unattributed_raw_text_results_are_not_counted_as_file_evidence() {
    // A long raw-text tool_result whose id maps to NO file_read
    // tool_use (e.g. a terminal/memory tool) must not masquerade as
    // file evidence.
    let long_text = "x".repeat(500);
    let result_row = tool_result("other-tool-1", &long_text, false);
    let s = summarize_evidence(vec![("tool", result_row.as_str())]);
    assert_eq!(s.file_read_calls, 0);
    assert_eq!(s.file_read_total_chars, 0);
    assert!(!s.is_sufficient());
}

#[test]
fn web_evidence_still_passes_without_any_file_reads() {
    // Regression guard: the pre-existing web path is unchanged.
    let search = serde_json::json!([{
        "type": "tool_result",
        "id": "t-1",
        "is_error": false,
        "content": [{"type": "text", "text":
            r#"{"backend":"duckduckgo_html","query":"q","result_count":3,"results":[]}"#}],
    }])
    .to_string();
    let extract = serde_json::json!([{
        "type": "tool_result",
        "id": "t-2",
        "is_error": false,
        "content": [{"type": "text", "text":
            r#"{"url":"https://example.com","title":"X","text":"...","char_count":1234,"truncated":false}"#}],
    }])
    .to_string();
    let s = summarize_evidence(vec![("tool", search.as_str()), ("tool", extract.as_str())]);
    assert!(s.is_sufficient());
    assert_eq!(s.file_read_calls, 0);
}
