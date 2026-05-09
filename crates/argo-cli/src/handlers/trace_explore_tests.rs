//! Unit tests for `trace_explore`. Six cases covering the brief's
//! acceptance matrix: empty, single root, parent+child, multi-root,
//! circular, JSON round-trip.
//!
//! These tests exercise the pure-Rust pipeline (load → build → render)
//! using temp NDJSON fixtures; the CLI surface (clap parse + dispatch)
//! is covered by `tests/trace_explore_smoke.rs`.

use super::*;
use std::io::Write;
use tempfile::TempDir;

fn write_fixture(dir: &TempDir, session: &str, body: &str) -> PathBuf {
    let path = dir.path().join(format!("{session}.ndjson"));
    let mut f = std::fs::File::create(&path).expect("create fixture");
    f.write_all(body.as_bytes()).expect("write fixture");
    path
}

fn span_line(
    span_id: &str,
    parent: Option<&str>,
    name: &str,
    start: u64,
    end: u64,
    status: &str,
) -> String {
    let parent_clause = match parent {
        Some(p) => format!(",\"parent_span_id\":\"{p}\""),
        None => String::new(),
    };
    format!(
        "{{\"span_id\":\"{span_id}\"{parent_clause},\"name\":\"{name}\",\"kind\":\"INTERNAL\",\
         \"start_unix_nano\":{start},\"end_unix_nano\":{end},\"status\":\"{status}\"}}\n"
    )
}

#[test]
fn empty_file_is_empty_tree_not_an_error() {
    let dir = TempDir::new().expect("tempdir");
    let path = write_fixture(&dir, "abc", "");
    let spans = load_spans(&path).expect("load");
    assert!(spans.is_empty());
    let roots = build_tree(spans);
    assert!(roots.is_empty());
}

#[test]
fn single_root_span_renders() {
    let body = span_line("a", None, "root", 0, 5_000_000, "ok");
    let dir = TempDir::new().expect("tempdir");
    let path = write_fixture(&dir, "s", &body);
    let spans = load_spans(&path).expect("load");
    let roots = build_tree(spans);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].span.name, "root");
    assert!(roots[0].children.is_empty());
    assert!((roots[0].span.duration_ms() - 5.0).abs() < 1e-9);
}

#[test]
fn parent_plus_child_renders_nested() {
    let mut body = String::new();
    body.push_str(&span_line("a", None, "root", 0, 10_000_000, "ok"));
    body.push_str(&span_line("b", Some("a"), "child", 1_000_000, 4_000_000, "ok"));
    let dir = TempDir::new().expect("tempdir");
    let path = write_fixture(&dir, "s", &body);
    let roots = build_tree(load_spans(&path).expect("load"));
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].children.len(), 1);
    assert_eq!(roots[0].children[0].span.name, "child");
    assert_eq!(roots[0].children[0].span.parent_span_id.as_deref(), Some("a"));
}

#[test]
fn multi_root_orphans_render_flat() {
    let mut body = String::new();
    // Two true roots:
    body.push_str(&span_line("a", None, "root_a", 0, 1_000, "ok"));
    body.push_str(&span_line("b", None, "root_b", 1_000, 2_000, "ok"));
    // Orphan: parent points at non-existent span.
    body.push_str(&span_line("c", Some("missing"), "orphan_c", 2_000, 3_000, "ok"));
    let dir = TempDir::new().expect("tempdir");
    let path = write_fixture(&dir, "s", &body);
    let roots = build_tree(load_spans(&path).expect("load"));
    assert_eq!(roots.len(), 3);
    let names: Vec<_> = roots.iter().map(|n| n.span.name.as_str()).collect();
    assert!(names.contains(&"root_a"));
    assert!(names.contains(&"root_b"));
    assert!(names.contains(&"orphan_c"));
    for r in &roots {
        assert!(r.children.is_empty());
    }
}

#[test]
fn circular_parent_does_not_infinite_loop() {
    // Cycle: a -> b -> a. Both reference each other; neither is a
    // true root. The walker must terminate (no panic, no hang) and
    // simply yield no roots — the cycle is structurally invalid OTLP.
    let mut body = String::new();
    body.push_str(&span_line("a", Some("b"), "a", 0, 100, "ok"));
    body.push_str(&span_line("b", Some("a"), "b", 0, 100, "ok"));
    let dir = TempDir::new().expect("tempdir");
    let path = write_fixture(&dir, "s", &body);
    let roots = build_tree(load_spans(&path).expect("load"));
    assert!(roots.is_empty());
}

#[test]
fn json_format_round_trips_through_serde_json() {
    let mut body = String::new();
    body.push_str(&span_line("a", None, "root", 0, 1_000_000, "ok"));
    body.push_str(&span_line("b", Some("a"), "child", 100_000, 500_000, "ok"));
    let dir = TempDir::new().expect("tempdir");
    let path = write_fixture(&dir, "s", &body);
    let roots = build_tree(load_spans(&path).expect("load"));
    let json = serde_json::to_string(&roots).expect("to_string");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("from_str");
    let arr = parsed.as_array().expect("top-level array");
    assert_eq!(arr.len(), 1);
    let root = &arr[0];
    assert_eq!(root["span"]["name"], "root");
    assert_eq!(root["children"].as_array().unwrap().len(), 1);
    assert_eq!(root["children"][0]["span"]["name"], "child");
}

#[test]
fn missing_file_returns_clear_error_not_panic() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("nope.ndjson");
    let err = load_spans(&path).expect_err("should fail");
    let msg = format!("{err}");
    assert!(msg.contains("trace explore"), "message: {msg}");
    assert!(msg.contains("nope.ndjson"), "message: {msg}");
}

#[test]
fn format_parse_accepts_known_values_rejects_others() {
    assert_eq!(TraceFormat::parse("").unwrap(), TraceFormat::Markdown);
    assert_eq!(TraceFormat::parse("markdown").unwrap(), TraceFormat::Markdown);
    assert_eq!(TraceFormat::parse("MD").unwrap(), TraceFormat::Markdown);
    assert_eq!(TraceFormat::parse("json").unwrap(), TraceFormat::Json);
    assert!(TraceFormat::parse("yaml").is_err());
}
