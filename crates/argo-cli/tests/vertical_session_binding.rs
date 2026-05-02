//! Integration tests for vertical session-binding (#651). Covers
//! both CLI (`argo new --vertical`, `argo session list --vertical`)
//! and gateway (`POST /verticals/<id>/sessions` + `POST /sessions`
//! with `vertical_id` in the body).
//!
//! All discovery is hermetic via `ARGO_VERTICALS_DIR`. ARGO_HOME
//! is a tempdir per test so the SQLite store is fresh.

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

// ---- CLI ------------------------------------------------------------

#[test]
fn cli_new_without_vertical_still_works() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "new",
            "--provider",
            "anthropic",
            "--model",
            "claude-haiku",
            "--title",
            "no-vertical",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("created session"), "got: {stdout}");
    assert!(!stdout.contains("vertical:"), "should not print vertical line");
}

#[test]
fn cli_new_with_valid_vertical_binds_it() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "new",
            "--provider",
            "anthropic",
            "--model",
            "claude-haiku",
            "--vertical",
            "demo",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("vertical:  demo"), "got: {stdout}");
}

#[test]
fn cli_new_with_unknown_vertical_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "new",
            "--provider",
            "anthropic",
            "--model",
            "claude-haiku",
            "--vertical",
            "ghost",
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "must fail");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "got: {stderr}");
    assert!(stderr.contains("not found"), "got: {stderr}");
}

#[test]
fn cli_session_list_includes_vertical_id_when_present() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    // Create one session bound + one unbound.
    for args in [
        vec![
            "new", "--provider", "anthropic", "--model", "haiku",
            "--title", "bound", "--vertical", "demo",
        ],
        vec![
            "new", "--provider", "anthropic", "--model", "haiku",
            "--title", "unbound",
        ],
    ] {
        let out = Command::new(argo_bin())
            .env("ARGO_HOME", home.path())
            .env("ARGO_VERTICALS_DIR", verticals.path())
            .args(&args)
            .output()
            .expect("spawn");
        assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    }

    // List as JSON; verify the bound row has vertical_id="demo"
    // and the unbound row has no vertical_id field.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["session", "list", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    let mut found_bound = false;
    let mut found_unbound = false;
    for line in body.lines() {
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line).expect("parse session row");
        match v["title"].as_str() {
            Some("bound") => {
                assert_eq!(v["vertical_id"], "demo", "bound row missing vertical_id");
                found_bound = true;
            }
            Some("unbound") => {
                // skip_serializing_if(Option::is_none) means the key
                // is absent for unbound rows.
                assert!(v.get("vertical_id").is_none(), "unbound row should omit vertical_id");
                found_unbound = true;
            }
            _ => {}
        }
    }
    assert!(found_bound && found_unbound, "missing rows in: {body}");
}

#[test]
fn cli_session_list_filter_by_vertical() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    for args in [
        vec!["new", "--provider", "p", "--model", "m", "--title", "bound", "--vertical", "demo"],
        vec!["new", "--provider", "p", "--model", "m", "--title", "unbound"],
    ] {
        Command::new(argo_bin())
            .env("ARGO_HOME", home.path())
            .env("ARGO_VERTICALS_DIR", verticals.path())
            .args(&args)
            .output()
            .expect("spawn");
    }
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["session", "list", "--vertical", "demo", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected only bound row, got {}: {body}", lines.len());
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["title"], "bound");
    assert_eq!(v["vertical_id"], "demo");
}

// ---- Gateway --------------------------------------------------------

fn spawn_gateway(
    port: u16,
    home: &Path,
    verticals_dir: &Path,
    max_requests: u32,
) -> (std::process::Child, BufReader<std::process::ChildStderr>) {
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
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
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);
    (child, stderr)
}

#[test]
fn gateway_post_verticals_id_sessions_creates_bound_row() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr) = spawn_gateway(port, home.path(), verticals.path(), 1);

    let resp = http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"haiku","provider":"anthropic","title":"gw-bound"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["vertical_id"], "demo");
    assert_eq!(v["title"], "gw-bound");
    let _ = child.wait();
}

#[test]
fn gateway_post_verticals_unknown_id_returns_404() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway(port, home.path(), verticals.path(), 1);

    let resp = http_post(
        port,
        "/verticals/ghost/sessions",
        r#"{"model":"m","provider":"p"}"#,
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");
    assert!(body_of(&resp).contains("ghost"));
    let _ = child.wait();
}

#[test]
fn gateway_legacy_post_sessions_still_works_unbound() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway(port, home.path(), verticals.path(), 1);

    let resp = http_post(
        port,
        "/sessions",
        r#"{"model":"m","provider":"p","title":"legacy"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    // No vertical was supplied -> vertical_id should be null/absent
    assert!(v["vertical_id"].is_null(), "vertical_id should be null: {v}");
    let _ = child.wait();
}

#[test]
fn gateway_legacy_post_sessions_with_body_vertical_id_binds_it() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr) = spawn_gateway(port, home.path(), verticals.path(), 1);

    let resp = http_post(
        port,
        "/sessions",
        r#"{"model":"m","provider":"p","title":"body-bound","vertical_id":"demo"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["vertical_id"], "demo");
    let _ = child.wait();
}

#[test]
fn gateway_routes_catalog_includes_post_verticals_sessions() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway(port, home.path(), verticals.path(), 1);

    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(
        body.contains("\"/verticals/<id>/sessions\""),
        "expected /verticals/<id>/sessions in catalog: {body}"
    );
    let _ = child.wait();
}
