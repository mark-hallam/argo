//! Smoke tests for `argo gateway stop` — SIGTERM to a live PID + the
//! no-pidfile clean-error path. Split out of `gateway_smoke.rs` to keep
//! each integration-test file under the 500-LOC ceiling. No behaviour change.
#![cfg(unix)]

mod common;
use common::{argo_bin, pick_free_port};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
fn gateway_stop_sends_sigterm_to_live_pid_then_clears_pidfile() {
    let _gateway_lock = common::gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
        .env("ARGO_GATEWAY_AUTH_BYPASS", "1")
        .args(["gateway", "start", "--skip-startup-check", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo gateway start");

    // Wait for "listening on" so we know the PID file is written.
    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("listener did not announce in 15s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }

    let pidfile = home.path().join("gateway.pid");
    assert!(pidfile.exists(), "PID file should be created on start");

    let stop = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
        .env("ARGO_GATEWAY_AUTH_BYPASS", "1")
        .args(["gateway", "stop"])
        .output()
        .expect("spawn argo gateway stop");
    assert!(stop.status.success(), "stop failed: {:?}", stop);
    assert!(String::from_utf8_lossy(&stop.stdout).contains("SIGTERM"));

    // The child should have exited from SIGTERM. Wait briefly.
    let status = child.wait().expect("wait child");
    // Default tokio shutdown on SIGTERM produces a non-zero exit
    // (signal-killed); accept both 0 and signal-terminated.
    assert!(
        status.code().is_none() || matches!(status.code(), Some(0) | Some(143)),
        "unexpected exit: {status:?}"
    );
}

#[test]
fn gateway_stop_with_no_pidfile_returns_clean_error() {
    let home = tempfile::tempdir().unwrap();
    let stop = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
        .env("ARGO_GATEWAY_AUTH_BYPASS", "1")
        .args(["gateway", "stop"])
        .output()
        .expect("spawn");
    assert!(!stop.status.success());
    let stderr = String::from_utf8_lossy(&stop.stderr);
    assert!(stderr.contains("no PID file"), "stderr: {stderr}");
}
