//! `argo trace explore` smoke test — runs the actual `argo` binary
//! against a hand-written `.ndjson` fixture in a temp directory.
//! Verifies the `--path` override + markdown render + JSON render +
//! the missing-file error path.

use std::process::Command;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

const FIXTURE_NDJSON: &str = concat!(
    r#"{"span_id":"a","name":"agent_loop","kind":"INTERNAL","start_unix_nano":0,"end_unix_nano":12300000,"status":"ok"}"#,
    "\n",
    r#"{"span_id":"b","parent_span_id":"a","name":"provider_call","kind":"CLIENT","start_unix_nano":1000000,"end_unix_nano":5000000,"status":"ok"}"#,
    "\n",
    r#"{"span_id":"c","parent_span_id":"a","name":"tool_call","kind":"INTERNAL","start_unix_nano":6000000,"end_unix_nano":11000000,"status":"ok"}"#,
    "\n",
);

#[test]
fn explore_renders_markdown_tree_from_fixture() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("smoke-session.ndjson"), FIXTURE_NDJSON)
        .expect("write fixture");
    let out = Command::new(argo_bin())
        .args([
            "trace",
            "explore",
            "smoke-session",
            "--path",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("spawn argo");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    assert!(stdout.contains("- agent_loop"), "stdout: {stdout}");
    assert!(stdout.contains("  - provider_call"), "stdout: {stdout}");
    assert!(stdout.contains("  - tool_call"), "stdout: {stdout}");
    // Provider call comes before tool call (sorted by start time).
    let p_idx = stdout.find("provider_call").expect("provider_call");
    let t_idx = stdout.find("tool_call").expect("tool_call");
    assert!(p_idx < t_idx, "ordering: {stdout}");
}

#[test]
fn explore_renders_json_when_format_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("smoke-session.ndjson"), FIXTURE_NDJSON)
        .expect("write fixture");
    let out = Command::new(argo_bin())
        .args([
            "trace",
            "explore",
            "smoke-session",
            "--format",
            "json",
            "--path",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("spawn argo");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("valid json output");
    let arr = parsed.as_array().expect("top-level array");
    assert_eq!(arr.len(), 1, "one root: {stdout}");
    assert_eq!(arr[0]["span"]["name"], "agent_loop");
    assert_eq!(arr[0]["children"].as_array().unwrap().len(), 2);
}

#[test]
fn explore_missing_session_file_errors_cleanly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = Command::new(argo_bin())
        .args([
            "trace",
            "explore",
            "no-such-session",
            "--path",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("spawn argo");
    assert!(!out.status.success(), "should fail on missing file");
    let stderr = String::from_utf8(out.stderr).expect("utf8");
    assert!(stderr.contains("trace explore"), "stderr: {stderr}");
    assert!(stderr.contains("no-such-session"), "stderr: {stderr}");
}
