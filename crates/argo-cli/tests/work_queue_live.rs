//! Live-API work-queue tests (#719 + #722). Gated by env flags
//! so CI never spends real API credit. Split out of
//! `tests/work_queue.rs` to keep both files under the per-file
//! LOC budget.
//!
//! Coverage:
//!   - `argo work serve --provider anthropic` end-to-end against
//!     the real Anthropic transport, with `--max-steps 1` so the
//!     v2 research-tools pre-flight doesn't intercept.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn fresh_workspace() -> (tempfile::TempDir, tempfile::TempDir) {
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
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    (home, work)
}

fn run_argo(home: &Path, work: &Path, args: &[&str]) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work)
        .output()
        .expect("spawn argo")
}

fn list_completed_jobs(home: &Path, work: &Path) -> Vec<Value> {
    let by_body = run_argo(
        home,
        work,
        &[
            "memory",
            "list",
            "--vertical",
            "gtm-engineer",
            "--body-contains",
            "completed_jobs",
            "--json",
        ],
    );
    assert!(
        by_body.status.success(),
        "{}",
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
                    if body.get("schema_key").and_then(|s| s.as_str()) == Some("completed_jobs") {
                        bodies.push(body);
                    }
                }
            }
        }
    }
    bodies
}

/// #719 live work-serve. Gated by BOTH `ARGO_LIVE_DEMO=1` AND
/// `ANTHROPIC_API_KEY` so CI never spends API credit. Exercises
/// `chat::run_turn` → Anthropic wire layer end-to-end. Uses
/// `--max-steps 1` so the v2 research-tools pre-flight (added
/// in #722, which denies stub web tools) doesn't intercept.
#[test]
#[cfg(not(target_os = "windows"))]
fn work_serve_live_anthropic_processes_one_job_when_opted_in() {
    let opted_in = std::env::var("ARGO_LIVE_DEMO")
        .map(|v| v == "1")
        .unwrap_or(false);
    let key_present = std::env::var("ANTHROPIC_API_KEY")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !opted_in || !key_present {
        eprintln!("SKIP: live work-serve requires ARGO_LIVE_DEMO=1 AND ANTHROPIC_API_KEY");
        return;
    }

    let (home, work) = fresh_workspace();
    let add = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "Briefly: what's one risk in scaling outbound campaigns?",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );

    // ANTHROPIC_API_KEY inherits via process env (not explicitly
    // forwarded), so the key never lands in any test-author log.
    let serve = Command::new(argo_bin())
        .args([
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--provider",
            "anthropic",
            "--model",
            "claude-sonnet-4-6",
            "--once",
            "--max-steps",
            "1",
            "--json",
        ])
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work.path())
        .output()
        .expect("spawn work serve --provider anthropic");

    assert!(
        serve.status.success(),
        "live work serve must succeed; exit={:?} stdout:\n{}\nstderr:\n{}",
        serve.status.code(),
        String::from_utf8_lossy(&serve.stdout),
        String::from_utf8_lossy(&serve.stderr)
    );

    let completed = list_completed_jobs(home.path(), work.path());
    assert_eq!(completed.len(), 1, "expected one completed_jobs row");
    let assistant_text = completed[0]["assistant_text"]
        .as_str()
        .expect("assistant_text");
    assert!(
        !assistant_text.starts_with("[mock]"),
        "live mode should produce real text, not a mock prefix; got: {assistant_text}"
    );
    assert!(
        !assistant_text.trim().is_empty(),
        "assistant_text must be non-empty"
    );
}
