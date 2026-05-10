//! Shared fixture helpers for `work_queue*.rs` integration tests.
//! Hoisted from the original 593-LOC `work_queue.rs` so that file
//! and its sibling `work_queue_visibility.rs` can share one set of
//! workspace-bootstrap + memory-list helpers.

#![allow(dead_code)]

use super::argo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// Spin up an isolated workspace + ARGO_HOME for one test; returns
/// (home_tempdir, work_tempdir). Both tempdirs auto-clean on drop.
pub fn fresh_workspace() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = Command::new(argo_bin())
        .arg("workspace")
        .arg("init")
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work.path())
        .output()
        .expect("workspace init");
    assert!(
        init.status.success(),
        "workspace init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    (home, work)
}

pub fn run_argo(home: &Path, work: &Path, args: &[&str]) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work)
        .output()
        .expect("spawn argo")
}

pub fn list_memory_jsonl(home: &Path, work: &Path, vertical: &str) -> Vec<Value> {
    let out = run_argo(
        home,
        work,
        &["memory", "list", "--vertical", vertical, "--json"],
    );
    assert!(
        out.status.success(),
        "memory list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Read every memory entry's body as JSON, filtered to a given
/// `schema_key`. Returns the parsed bodies (the actual job
/// payloads, not the wrapping memory-row metadata).
///
/// `argo memory list --json` returns row metadata only (no body),
/// so we filter to candidate rows via `--body-contains <schema_key>`,
/// then `argo memory view <id> --json` fetches each body.
pub fn list_jobs_by_schema(
    home: &Path,
    work: &Path,
    vertical: &str,
    schema_key: &str,
) -> Vec<Value> {
    let by_body = run_argo(
        home,
        work,
        &[
            "memory",
            "list",
            "--vertical",
            vertical,
            "--body-contains",
            schema_key,
            "--json",
        ],
    );
    assert!(
        by_body.status.success(),
        "memory list --body-contains failed: {}",
        String::from_utf8_lossy(&by_body.stderr)
    );
    let rows: Vec<Value> = String::from_utf8_lossy(&by_body.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let mut bodies = Vec::new();
    for row in rows {
        let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let view = run_argo(home, work, &["memory", "view", id, "--json"]);
        if !view.status.success() {
            continue;
        }
        if let Ok(v) = serde_json::from_slice::<Value>(&view.stdout) {
            if let Some(body_str) = v.get("body").and_then(|x| x.as_str()) {
                if let Ok(body) = serde_json::from_str::<Value>(body_str) {
                    if body.get("schema_key").and_then(|s| s.as_str()) == Some(schema_key) {
                        bodies.push(body);
                    }
                }
            }
        }
    }
    bodies
}
