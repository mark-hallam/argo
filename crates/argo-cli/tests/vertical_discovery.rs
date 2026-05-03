//! Integration tests for the vertical-template discovery surface
//! (#650). Covers BOTH the CLI (`argo vertical list/show/validate`)
//! and the gateway (`GET /verticals` + `GET /verticals/<id>`).
//!
//! Discovery is overridden via `ARGO_VERTICALS_DIR` so each test
//! sees a hermetic fixture. No production-data side effects.

mod common;

use common::{argo_bin, await_listening, body_of, http_get, pick_free_port};
use std::io::{BufReader, Write};
use std::process::{Command, Stdio};

const MIN_TOML: &str = "schema_version = \"1\"\n\n\
[vertical]\n\
id = \"demo\"\n\
name = \"Demo Vertical\"\n\
description = \"Smoke test fixture\"\n\
version = \"0.1.0\"\n\
tags = [\"test\"]\n";

fn write_demo_vertical(verticals_dir: &std::path::Path) {
    let demo = verticals_dir.join("demo");
    std::fs::create_dir_all(&demo).unwrap();
    std::fs::write(demo.join("vertical.toml"), MIN_TOML).unwrap();
}

fn write_broken_vertical(verticals_dir: &std::path::Path) {
    let bad = verticals_dir.join("broken");
    std::fs::create_dir_all(&bad).unwrap();
    // Missing the required [vertical] table entirely.
    std::fs::write(bad.join("vertical.toml"), "schema_version = \"1\"\n").unwrap();
}

// ---- CLI tests ------------------------------------------------------

#[test]
fn cli_list_empty_dir_returns_no_verticals_marker() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    std::fs::create_dir_all(&verticals).unwrap();

    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .args(["vertical", "list"])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("(no verticals"), "got: {stdout}");
}

#[test]
fn cli_list_json_with_one_vertical() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_demo_vertical(&verticals);

    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .args(["vertical", "list", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).expect("parse");
    assert_eq!(v["ok"], true);
    let loaded = v["loaded"].as_array().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0]["id"], "demo");
    assert_eq!(loaded[0]["version"], "0.1.0");
    assert!(v["errors"].as_array().unwrap().is_empty());
}

#[test]
fn cli_list_json_surfaces_per_vertical_errors_without_failing() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_demo_vertical(&verticals);
    write_broken_vertical(&verticals);

    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .args(["vertical", "list", "--json"])
        .output()
        .expect("spawn");
    // List exits 0 even when individual verticals failed — the
    // catalogue is still partially useful.
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["loaded"].as_array().unwrap().len(), 1);
    assert_eq!(v["errors"].as_array().unwrap().len(), 1);
}

#[test]
fn cli_show_json_returns_full_summary() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_demo_vertical(&verticals);

    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .args(["vertical", "show", "demo", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["id"], "demo");
    assert_eq!(v["name"], "Demo Vertical");
    assert_eq!(v["description"], "Smoke test fixture");
}

#[test]
fn cli_show_markdown_renders_h1_with_id_and_name() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_demo_vertical(&verticals);

    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .args(["vertical", "show", "demo", "--format", "markdown"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(body.starts_with("# Demo Vertical (`demo`)"), "got: {body}");
}

#[test]
fn cli_show_missing_id_exits_non_zero() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    std::fs::create_dir_all(&verticals).unwrap();

    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .args(["vertical", "show", "ghost"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "missing id must exit non-zero");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "got: {stderr}");
    assert!(stderr.contains("not found"), "got: {stderr}");
}

#[test]
fn cli_validate_passes_for_valid_vertical() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_demo_vertical(&verticals);

    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .args(["vertical", "validate", "demo", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["id"], "demo");
    assert_eq!(v["valid"], true);
}

#[test]
fn cli_validate_fails_loud_for_broken_vertical() {
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_broken_vertical(&verticals);

    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .args(["vertical", "validate", "broken", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "broken vertical must fail validate");
    let body = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["id"], "broken");
    assert_eq!(v["valid"], false);
    assert!(!v["error"].as_str().unwrap().is_empty());
}

// ---- Gateway tests --------------------------------------------------

/// Spawn a gateway with the given verticals dir + ARGO_HOME, then
/// block until it announces "listening on …". Returns the child
/// AND the stderr reader — the caller MUST hold both for the
/// process's full lifetime, otherwise dropping the stderr pipe
/// SIGPIPEs the gateway on its next stderr write. (Discovered the
/// hard way while writing this test file.)
fn spawn_gateway_with_verticals_dir(
    port: u16,
    home: &std::path::Path,
    verticals_dir: &std::path::Path,
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
fn gateway_verticals_list_returns_loaded() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_demo_vertical(&verticals);

    let (mut child, _stderr) = spawn_gateway_with_verticals_dir(port, home.path(), &verticals, 1);

    let resp = http_get(port, "/verticals");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("body parses");
    assert_eq!(v["ok"], true);
    assert_eq!(v["loaded"].as_array().unwrap().len(), 1);
    assert_eq!(v["loaded"][0]["id"], "demo");

    let _ = child.wait();
}

#[test]
fn gateway_verticals_list_markdown() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_demo_vertical(&verticals);

    let (mut child, _stderr) = spawn_gateway_with_verticals_dir(port, home.path(), &verticals, 1);

    let resp = http_get(port, "/verticals?format=markdown");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(body_of(&resp).contains("# Verticals"));

    let _ = child.wait();
}

#[test]
fn gateway_verticals_show_returns_summary() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let verticals = dir.path().join("verticals");
    write_demo_vertical(&verticals);

    let (mut child, _stderr) = spawn_gateway_with_verticals_dir(port, home.path(), &verticals, 1);

    let resp = http_get(port, "/verticals/demo");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["id"], "demo");
    assert_eq!(v["name"], "Demo Vertical");

    let _ = child.wait();
}

#[test]
fn gateway_verticals_show_returns_404_for_missing() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway_with_verticals_dir(port, home.path(), verticals.path(), 1);

    let resp = http_get(port, "/verticals/ghost");
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");
    assert!(body_of(&resp).contains("not found"));

    let _ = child.wait();
}

#[test]
fn gateway_routes_catalog_lists_vertical_endpoints() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway_with_verticals_dir(port, home.path(), verticals.path(), 1);

    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(
        body.contains("\"/verticals\""),
        "/routes should include /verticals: {body}"
    );
    assert!(
        body.contains("\"/verticals/<id>\""),
        "/routes should include /verticals/<id>: {body}"
    );

    let _ = child.wait();
}

// Silence unused-import warnings for helpers that other test bins
// exercise.
#[allow(dead_code)]
fn _quiet(_w: &mut std::io::Stdout) {
    let _ = _w.write_all(b"");
}
