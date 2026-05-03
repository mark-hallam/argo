//! Gateway integration tests for `GET /verticals/<id>/lint`
//! (#672). Mirrors `vertical_lint.rs` (the CLI surface) but
//! drives the same behaviour over HTTP.

mod common;

use common::{argo_bin, body_of, http_get, pick_free_port};
use std::path::Path;
use std::process::Command;

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

fn repo_verticals_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals")
}

#[test]
fn gateway_get_lint_pr_checker_returns_200_ok_no_errors() {
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) =
        boot_with_vertical(&repo_verticals_dir(), wd.path(), 2);

    let resp = http_get(port, "/verticals/pr-checker/lint");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["id"], "pr-checker");
    assert_eq!(v["ok"], true);
    // Per-finding check — anything present must NOT be an error.
    for f in v["findings"].as_array().unwrap_or(&Vec::new()) {
        let sev = f["severity"].as_str().unwrap();
        assert_ne!(sev, "error", "shipped pr-checker emitted error: {f}");
    }

    let _ = child.kill();
}

#[test]
fn gateway_get_lint_receptionist_returns_200_ok_no_errors() {
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) =
        boot_with_vertical(&repo_verticals_dir(), wd.path(), 2);

    let resp = http_get(port, "/verticals/receptionist/lint");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], true);

    let _ = child.kill();
}

#[test]
fn gateway_get_lint_unknown_vertical_returns_404() {
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) =
        boot_with_vertical(verticals.path(), wd.path(), 1);

    let resp = http_get(port, "/verticals/ghost/lint");
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp: {resp}");
    let body = body_of(&resp);
    assert!(body.contains("ghost"), "body: {body}");

    let _ = child.kill();
}

#[test]
fn gateway_get_lint_synthetic_typo_emits_warn_finding() {
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    let v = verticals.path().join("typo");
    std::fs::create_dir_all(&v).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"typo\"\nname = \"T\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\", \"file_reed_typo\"]\ndenied = []\n",
    )
    .unwrap();
    let (mut child, _stderr, port) =
        boot_with_vertical(verticals.path(), wd.path(), 1);

    let resp = http_get(port, "/verticals/typo/lint");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], true, "warn-only must keep ok=true");
    let warn = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == "tools.allowed.unknown_name")
        .expect("expected warn finding");
    assert_eq!(warn["severity"], "warn");
    assert!(warn["message"]
        .as_str()
        .unwrap()
        .contains("file_reed_typo"));

    let _ = child.kill();
}

#[test]
fn gateway_routes_catalog_lists_lint_endpoint() {
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) =
        boot_with_vertical(&repo_verticals_dir(), wd.path(), 1);

    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"));
    let body = body_of(&resp);
    assert!(
        body.contains("/verticals/<id>/lint"),
        "catalog should list lint route; body: {body}"
    );

    let _ = child.kill();
}
