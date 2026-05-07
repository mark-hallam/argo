//! Helpers for the operator-perspective product e2e test
//! (Stream `claude-end-to-end-product-test`).
//!
//! Distinct from `e2e_v1_demo.rs`, which exercises internal library
//! APIs. This pair drives the actual `argo` binary end-to-end so a
//! reviewer can read the test and follow the operator's experience
//! one stdin keystroke at a time.

#![allow(dead_code)]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::argo_bin;

/// Where the test fixtures live relative to the argo-cli crate.
#[must_use]
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Run `argo <args>` against the given home + cwd. Returns the
/// raw [`Output`] so callers can assert on stderr text on failure.
pub fn run_argo(home: &Path, work: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(work)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

/// Assert the command succeeded; on failure print which step the
/// caller is on plus the captured stdout/stderr so the operator
/// can debug from CI logs alone.
pub fn assert_success(label: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{label} failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Run a JSON-emitting `argo` command and parse its stdout. Panics
/// (with full output) if the command failed or stdout is not JSON.
pub fn json_cmd(label: &str, home: &Path, work: &Path, args: &[&str]) -> Value {
    let out = run_argo(home, work, args);
    assert_success(label, &out);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{label} stdout is not JSON: {e}\nstdout:\n{}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

/// Inject a worker_action memory row through the public CLI surface
/// (`argo memory append --vertical <id> memory <body>`). Returns
/// the action_id stamped into the row so the caller can reference
/// it through subsequent `argo worker actions ...` calls.
pub fn inject_pending_action(
    label: &str,
    home: &Path,
    work: &Path,
    vertical: &str,
    payload_template: &Value,
) -> String {
    let action_id = uuid::Uuid::now_v7().to_string();
    let mut body = payload_template.clone();
    body["action_id"] = Value::String(action_id.clone());
    body["state"] = Value::String("pending".to_string());
    body["created_at"] = Value::String(chrono::Utc::now().to_rfc3339());
    let body_str = serde_json::to_string(&body).expect("serialize action body");
    let out = run_argo(
        home,
        work,
        &[
            "memory",
            "append",
            "--vertical",
            vertical,
            "memory",
            &body_str,
        ],
    );
    assert_success(&format!("{label} (memory append)"), &out);
    action_id
}

/// Read the latest persisted state for an action via
/// `argo worker actions show <id> --json`. The handler returns the
/// serialized [`ActionRecord`] at the top level, so state lives at
/// `/state` (not under `/action/...`).
pub fn current_action_state(home: &Path, work: &Path, action_id: &str) -> String {
    let v = json_cmd(
        "worker actions show",
        home,
        work,
        &["worker", "actions", "show", action_id, "--json"],
    );
    v.get("state")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            panic!(
                "worker actions show output missing top-level `state`\nfull: {}",
                serde_json::to_string_pretty(&v).unwrap_or_default()
            )
        })
}
