//! Gateway integration tests for vertical-aware memory (#687).
//! Mirrors `memory_vertical.rs` (the CLI surface from #686)
//! but drives the same behaviour over HTTP.

mod common;

use common::{argo_bin, body_of, boot, http_get, http_post, pick_free_port};
use std::process::Command;

/// Boot gateway, init a workspace via the CLI side first (so
/// `first_workspace()` returns Some). Returns (child, stderr, port).
fn boot_with_workspace(
    home: &std::path::Path,
    workspace_root: &std::path::Path,
    max_requests: u32,
) -> (
    std::process::Child,
    std::io::BufReader<std::process::ChildStderr>,
    u16,
) {
    // Initialize the workspace synchronously via the CLI before
    // booting — gateway routes need a registered workspace.
    let init = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(workspace_root)
        .args(["workspace", "init"])
        .output()
        .expect("spawn workspace init");
    assert!(
        init.status.success(),
        "workspace init: {}",
        String::from_utf8_lossy(&init.stderr)
    );

    let port = pick_free_port();
    // boot() helper uses its own ARGO_HOME tempdir, so for this
    // test we set ARGO_HOME via env before invoking gateway.
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(workspace_root)
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

/// Suppress "unused" warning: `boot` is imported by the
/// shared common module via wildcard but not all tests use it.
#[allow(dead_code)]
fn _hint_boot_in_use() {
    let _ = boot;
}

#[test]
fn gateway_post_memory_with_vertical_id_tags_the_row() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_workspace(home.path(), root.path(), 4);

    let resp = http_post(
        port,
        "/memory",
        r#"{"file":"memory","body":"alpha-row","vertical_id":"alpha"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["vertical_id"], "alpha");

    // GET /memory?vertical=alpha → exactly the alpha-tagged row.
    let resp = http_get(port, "/memory?vertical=alpha");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let arr: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["vertical_id"], "alpha");

    let _ = child.kill();
}

#[test]
fn gateway_get_memory_vertical_excludes_other_verticals_rows() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_workspace(home.path(), root.path(), 6);

    http_post(
        port,
        "/memory",
        r#"{"file":"memory","body":"alpha","vertical_id":"alpha"}"#,
    );
    http_post(
        port,
        "/memory",
        r#"{"file":"memory","body":"beta","vertical_id":"beta"}"#,
    );

    let resp = http_get(port, "/memory?vertical=alpha");
    let arr: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 1, "alpha view must contain exactly its row");
    assert_eq!(arr[0]["vertical_id"], "alpha");

    let resp = http_get(port, "/memory?vertical=beta");
    let arr: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["vertical_id"], "beta");

    let _ = child.kill();
}

#[test]
fn gateway_get_memory_vertical_excludes_legacy_null_rows() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_workspace(home.path(), root.path(), 4);

    // Legacy untagged write.
    http_post(
        port,
        "/memory",
        r#"{"file":"memory","body":"legacy-row"}"#,
    );

    // ?vertical=alpha → empty (default-deny).
    let resp = http_get(port, "/memory?vertical=alpha");
    let arr: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let arr = arr.as_array().unwrap();
    assert!(
        arr.is_empty(),
        "vertical scope must exclude legacy NULL row; got {arr:?}"
    );

    // Plain GET /memory still returns it (back-compat).
    let resp = http_get(port, "/memory");
    let arr: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 1, "legacy view must keep showing NULL rows");
    assert!(arr[0]["vertical_id"].is_null());

    let _ = child.kill();
}

#[test]
fn gateway_get_memory_vertical_unknown_id_returns_empty() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_workspace(home.path(), root.path(), 3);

    http_post(
        port,
        "/memory",
        r#"{"file":"memory","body":"x","vertical_id":"alpha"}"#,
    );

    let resp = http_get(port, "/memory?vertical=ghost");
    let arr: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let arr = arr.as_array().unwrap();
    assert!(arr.is_empty(), "ghost vertical must return empty; got {arr:?}");

    let _ = child.kill();
}

#[test]
fn gateway_routes_catalog_documents_vertical_query_param() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_workspace(home.path(), root.path(), 1);

    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"));
    let body = body_of(&resp);
    assert!(
        body.contains("vertical_id"),
        "POST /memory catalog row should mention vertical_id; body: {body}"
    );
    assert!(
        body.contains("vertical=<id>"),
        "GET /memory catalog row should mention ?vertical=<id>; body: {body}"
    );

    let _ = child.kill();
}
