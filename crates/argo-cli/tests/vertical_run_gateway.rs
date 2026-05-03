//! Gateway integration tests for `POST /verticals/<id>/run`
//! (#670). Mirrors `vertical_run.rs` (the CLI surface) but
//! drives the same behaviour over HTTP so non-CLI clients —
//! web UI, scripted automation, CI runners — get parity.

mod common;

use common::{argo_bin, body_of, http_get, http_post, pick_free_port};
use std::path::Path;
use std::process::Command;

/// Boot the gateway with the workspace's `verticals/` dir as
/// the discovery root. Returns (child, stderr, port).
fn boot_with_vertical(
    vertical_root: &Path,
    workspace_cwd: &Path,
    max_requests: u32,
) -> (
    std::process::Child,
    std::io::BufReader<std::process::ChildStderr>,
    u16,
) {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap().keep();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", vertical_root)
        .current_dir(workspace_cwd)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let mut stderr = std::io::BufReader::new(child.stderr.take().unwrap());
    common::await_listening(&mut stderr);
    (child, stderr, port)
}

fn write_demo_vertical_with_workflow(verticals_dir: &Path, id: &str) {
    let v = verticals_dir.join(id);
    std::fs::create_dir_all(v.join("workflows")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [tools]\nallowed = [\"file_read\"]\ndenied = []\n\
             [workflows]\nfiles = [\"workflows/main.workflow.toml\"]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        v.join("workflows/main.workflow.toml"),
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"hello.txt\" }\n",
    )
    .unwrap();
}

#[test]
fn gateway_post_run_composite_creates_session_runs_workflow_returns_201() {
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    std::fs::write(wd.path().join("hello.txt"), "from gateway run").unwrap();
    write_demo_vertical_with_workflow(verticals.path(), "demo");
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 4);

    let resp = http_post(
        port,
        "/verticals/demo/run",
        r#"{"workflow":"go","title":"gw composite","provider":"mock","model":"mock"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["vertical_id"], "demo");
    let sid = v["session_id"]
        .as_str()
        .expect("session_id in response body");
    let wf = &v["workflow"];
    assert_eq!(wf["ok"], true);
    assert_eq!(wf["workflow"], "go");
    assert_eq!(wf["steps"].as_array().unwrap().len(), 1);
    assert_eq!(wf["steps"][0]["status"], "ok");
    assert!(wf["steps"][0]["message"]
        .as_str()
        .unwrap()
        .contains("from gateway run"));

    // The session row exists + carries the audit rows from
    // the workflow run.
    let show = http_get(port, &format!("/sessions/{sid}"));
    assert!(show.contains("HTTP/1.1 200 OK"), "show resp: {show}");

    let _ = child.kill();
}

#[test]
fn gateway_post_run_unknown_vertical_returns_404() {
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 1);

    let resp = http_post(port, "/verticals/ghost/run", r#"{"workflow":"any"}"#);
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp: {resp}");
    let body = body_of(&resp);
    assert!(body.contains("ghost"), "body: {body}");

    let _ = child.kill();
}

#[test]
fn gateway_post_run_missing_workflow_field_returns_400() {
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    write_demo_vertical_with_workflow(verticals.path(), "demo");
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 1);

    let resp = http_post(port, "/verticals/demo/run", r#"{}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp: {resp}");
    let body = body_of(&resp);
    assert!(body.contains("missing `workflow`"), "body: {body}");

    let _ = child.kill();
}

#[test]
fn gateway_post_run_returns_200_with_ok_false_when_a_step_fails() {
    // Workflow references a missing file → file_read returns
    // is_error=true → step status="failed" → workflow.ok=false.
    // The endpoint returns 200 (run completed; surface the
    // partial result) rather than 5xx (server error).
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    let v = verticals.path().join("demo");
    std::fs::create_dir_all(v.join("workflows")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"X\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\"]\ndenied = []\n\
         [workflows]\nfiles = [\"workflows/main.workflow.toml\"]\n",
    )
    .unwrap();
    std::fs::write(
        v.join("workflows/main.workflow.toml"),
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"missing.txt\" }\n",
    )
    .unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 2);

    let resp = http_post(port, "/verticals/demo/run", r#"{"workflow":"go"}"#);
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["workflow"]["ok"], false);

    let _ = child.kill();
}

#[test]
fn gateway_routes_catalog_lists_verticals_run_endpoint() {
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 1);
    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let body = body_of(&resp);
    assert!(
        body.contains("/verticals/<id>/run"),
        "catalog should list verticals run route; body: {body}"
    );
    let _ = child.kill();
}
