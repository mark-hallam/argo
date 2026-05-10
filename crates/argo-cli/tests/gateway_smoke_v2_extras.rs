//! Read-only listing endpoints split out of `gateway_smoke_v2.rs`
//! so that file stays under the 500-LOC budget. These three smoke
//! tests share the "endpoint returns a JSON array on a clean DB"
//! shape and don't need the same boot fixture as the streaming /
//! mutating tests still in `gateway_smoke_v2.rs`.
mod common;
use common::{argo_bin, await_listening, body_of, http_get, pick_free_port};

use std::io::BufReader;
use std::process::{Command, Stdio};

#[test]
fn forge_runs_returns_empty_array_on_clean_db() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_get(port, "/forge/runs");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.starts_with('['), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn routes_endpoint_lists_known_paths() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.starts_with('['), "body={body}");
    // Spot-check a handful of expected paths.
    for path in ["/health", "/sessions", "/tools", "/routes"] {
        assert!(body.contains(path), "body missing {path}: {body}");
    }

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn workspace_artifacts_returns_empty_array_on_clean_db() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_get(port, "/workspace/artifacts");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.starts_with('['), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}
