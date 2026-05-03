//! `GET /audit/recent` and `GET /audit/sessions/<id>` carry the
//! per-row `vertical_id` joined from `session.vertical_id`
//! (#691). Pairs with the by-vertical histogram (#688) — the
//! rollup tells you which vertical is busy, the per-row enrichment
//! lets you triage which row belongs to which vertical without a
//! second query.
//!
//! Coverage:
//! - JSON shape includes `vertical_id` for bound + null/absent
//!   for unbound (the field is `skip_serializing_if=Option::is_none`)
//! - `?format=markdown` for `/audit/recent` shows a `vertical`
//!   column
//! - `?format=markdown` for `/audit/sessions/<id>` includes the
//!   bound vertical in the heading suffix

mod common;

use common::{argo_bin, await_listening, body_of, http_get, http_post, pick_free_port};
use std::io::BufReader;
use std::path::Path;
use std::process::{Command, Stdio};

const MIN_TOML: &str = "schema_version = \"1\"\n\n\
[vertical]\n\
id = \"demo\"\n\
name = \"Demo\"\n\
version = \"0.1.0\"\n";

fn write_demo_vertical(verticals_dir: &Path) {
    let demo = verticals_dir.join("demo");
    std::fs::create_dir_all(&demo).unwrap();
    std::fs::write(demo.join("vertical.toml"), MIN_TOML).unwrap();
}

fn workspace_init(home: &Path, root: &Path) {
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(root)
        .args(["workspace", "init"])
        .output()
        .expect("spawn workspace init");
    assert!(
        out.status.success(),
        "workspace init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn boot_gateway(
    home: &Path,
    workspace_root: &Path,
    verticals: &Path,
    max_requests: u32,
) -> (
    std::process::Child,
    BufReader<std::process::ChildStderr>,
    u16,
) {
    workspace_init(home, workspace_root);
    let port = pick_free_port();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .current_dir(workspace_root)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);
    (child, stderr, port)
}

/// Grant an approval — writes an `ApprovalGranted` audit row,
/// which gives us session-tagged audit traffic to inspect
/// without needing a real LLM call.
fn grant_approval(port: u16, sid: &str, tool: &str) {
    let resp = http_post(
        port,
        &format!("/sessions/{sid}/approve"),
        &format!("{{\"tool\":\"{tool}\"}}"),
    );
    assert!(
        resp.contains("HTTP/1.1 200") || resp.contains("HTTP/1.1 201"),
        "approve: {resp}"
    );
}

#[test]
fn audit_recent_json_carries_vertical_id_for_bound_and_unbound_rows() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr, port) =
        boot_gateway(home.path(), root.path(), verticals.path(), 10);

    // Bound + unbound session.
    let resp = http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"bound"}"#,
    );
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let bound_sid = v["id"].as_str().unwrap().to_string();

    let resp = http_post(
        port,
        "/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"unbound"}"#,
    );
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let unbound_sid = v["id"].as_str().unwrap().to_string();

    // Generate at least one audit row per session via /steer.
    grant_approval(port, &bound_sid, "delegate_task");
    grant_approval(port, &unbound_sid, "delegate_task");

    let resp = http_get(port, "/audit/recent?limit=20");
    assert!(resp.contains("HTTP/1.1 200 OK"), "recent: {resp}");
    let arr: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let rows = arr.as_array().unwrap();

    // At least one row should have vertical_id == "demo" — the
    // steer for the bound session. At least one should be missing
    // the field (or null) — the steer for the unbound session.
    let has_demo = rows.iter().any(|r| r["vertical_id"] == "demo");
    let has_unbound = rows
        .iter()
        .any(|r| r.get("vertical_id").is_none() || r["vertical_id"].is_null());
    assert!(has_demo, "expected a demo-tagged audit row in:\n{rows:?}");
    assert!(
        has_unbound,
        "expected an unbound (null/absent) vertical_id row in:\n{rows:?}"
    );

    let _ = child.kill();
}

#[test]
fn audit_recent_markdown_includes_vertical_column() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr, port) =
        boot_gateway(home.path(), root.path(), verticals.path(), 8);

    let resp = http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"md"}"#,
    );
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let sid = v["id"].as_str().unwrap().to_string();
    grant_approval(port, &sid, "delegate_task");

    let resp = http_get(port, "/audit/recent?limit=20&format=markdown");
    assert!(resp.contains("HTTP/1.1 200 OK"), "recent md: {resp}");
    let body = body_of(&resp);
    assert!(
        body.contains("| vertical |"),
        "missing vertical header column; got:\n{body}"
    );
    assert!(
        body.contains("`demo`"),
        "missing demo cell in markdown table; got:\n{body}"
    );

    let _ = child.kill();
}

#[test]
fn audit_session_chain_markdown_includes_vertical_in_heading() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr, port) =
        boot_gateway(home.path(), root.path(), verticals.path(), 6);

    let resp = http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"chain"}"#,
    );
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let sid = v["id"].as_str().unwrap().to_string();
    grant_approval(port, &sid, "delegate_task");

    let resp = http_get(port, &format!("/audit/sessions/{sid}?format=markdown"));
    assert!(resp.contains("HTTP/1.1 200 OK"), "chain md: {resp}");
    let body = body_of(&resp);
    assert!(
        body.contains("vertical `demo`"),
        "heading missing vertical suffix; got:\n{body}"
    );

    let _ = child.kill();
}
