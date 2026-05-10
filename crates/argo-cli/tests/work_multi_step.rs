//! End-to-end tests for Worker v2 multi-step execution (#722).
//!
//! Mock-mode only — no API key required.
//!
//! Coverage (this file):
//!   - `argo work serve` against the production gtm-engineer
//!     manifest (#723 made it allow real web tools) PASSES the
//!     pre-flight gate and runs the mock turn
//!   - `argo work serve --max-steps 1` bypasses the pre-flight
//!     and runs the single-step mock path; mock now emits the
//!     four labelled sections + `[STOP: TASK_COMPLETE]` so the
//!     splitter materialises research/pricing/listings/checklist
//!   - completed_jobs body carries `steps_completed` and
//!     `stop_reason` (the v2 outcome shape)
//!   - artifact bundle includes the four section files (v1 set
//!     of transcript/assistant_output/job/result still present)
//!
//! The pre-flight failure path (`research_tools_unavailable`)
//! lives in the sibling `work_multi_step_preflight.rs`.

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
        "workspace init failed: {}",
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

/// Same body-fetcher pattern as `tests/work_queue.rs`. Inlined
/// rather than shared via a helper module to keep test files
/// independent.
fn list_jobs_by_schema(home: &Path, work: &Path, schema_key: &str) -> Vec<Value> {
    let by_body = run_argo(
        home,
        work,
        &[
            "memory",
            "list",
            "--vertical",
            "gtm-engineer",
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

// ────────────────────────────────────────────────────────── tests

/// #723 — with the production gtm-engineer manifest (which now
/// allows real web_search / web_extract), the default
/// multi-step worker run must PASS the pre-flight gate and
/// dispatch the mock turn. Locks the post-#723 happy path so a
/// future regression that re-denies the tools fails this test
/// loudly.
#[test]
fn multi_step_pre_flight_passes_when_research_tools_are_real() {
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
            "Smoke test that pre-flight passes.",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );

    // Default --max-steps 5 → pre-flight runs. With the production
    // gtm-engineer manifest post-#723 it must PASS, then the mock
    // turn runs and emits [STOP: TASK_COMPLETE] on step 1.
    let serve = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--once",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "{}",
        String::from_utf8_lossy(&serve.stderr)
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    assert_eq!(processed["ok"], true);
    assert_eq!(
        processed["stop_reason"], "task_complete",
        "post-#723 pre-flight passes; mock then completes in 1 step"
    );
    assert_eq!(processed["steps_completed"], 1);
}

/// `--max-steps 1` skips the pre-flight check and runs the
/// single-step mock path. The mock now emits the four labelled
/// sections + `[STOP: TASK_COMPLETE]`, so the artifact splitter
/// materialises all four section files alongside the v1
/// transcript/job/result bundle.
#[test]
fn single_step_mock_writes_full_section_artifact_set() {
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
            "Diagnose the 100/2/0 cohort.",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );

    let serve = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--once",
            "--max-steps",
            "1",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "{}",
        String::from_utf8_lossy(&serve.stderr)
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    assert_eq!(processed["ok"], true);
    assert_eq!(processed["stop_reason"], "task_complete");
    assert_eq!(processed["steps_completed"], 1);

    let output_dir = PathBuf::from(
        processed["output_dir"]
            .as_str()
            .expect("output_dir on serve --json"),
    );

    // v1 bundle still lands.
    for f in [
        "transcript.md",
        "assistant_output.md",
        "job.json",
        "result.json",
    ] {
        assert!(output_dir.join(f).exists(), "v1 artifact missing: {f}");
    }

    // v2 bundle: four labelled section files materialised by the splitter.
    for f in [
        "research.md",
        "pricing.md",
        "listings.md",
        "approval_checklist.md",
    ] {
        assert!(output_dir.join(f).exists(), "v2 section file missing: {f}");
    }

    let research = std::fs::read_to_string(output_dir.join("research.md")).unwrap();
    let pricing = std::fs::read_to_string(output_dir.join("pricing.md")).unwrap();
    let listings = std::fs::read_to_string(output_dir.join("listings.md")).unwrap();
    let approval = std::fs::read_to_string(output_dir.join("approval_checklist.md")).unwrap();

    assert!(
        research.contains("research"),
        "research.md must include the section header"
    );
    assert!(
        pricing.contains("pricing"),
        "pricing.md must include the section header"
    );
    assert!(
        listings.contains("listings"),
        "listings.md must include the section header"
    );
    assert!(approval.contains("Approval checklist"));
    assert!(approval.contains("stop_reason: `task_complete`"));

    // job.json carries the v2 outcome fields.
    let job_json: Value =
        serde_json::from_slice(&std::fs::read(output_dir.join("job.json")).unwrap()).unwrap();
    assert_eq!(job_json["steps_completed"], 1);
    assert_eq!(job_json["stop_reason"], "task_complete");
    assert!(job_json["stop_detail"].as_str().is_some());

    // result.json carries pointers to the new section files.
    let result_json: Value =
        serde_json::from_slice(&std::fs::read(output_dir.join("result.json")).unwrap()).unwrap();
    for k in [
        "research_path",
        "pricing_path",
        "listings_path",
        "approval_checklist_path",
    ] {
        assert!(
            result_json[k].as_str().is_some(),
            "result.json missing {k}; got {result_json}"
        );
    }

    // completed_jobs body carries v2 outcome fields too.
    let completed = list_jobs_by_schema(home.path(), work.path(), "completed_jobs");
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0]["steps_completed"], 1);
    assert_eq!(completed[0]["stop_reason"], "task_complete");
}

/// `--max-steps 1` is the explicit single-step mode; output_dir
/// keeps landing under `.skycot/outputs/<job_id>/` so existing
/// operator habits aren't broken.
#[test]
fn output_dir_layout_is_stable_across_v1_and_v2_modes() {
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
            "smoke",
            "--json",
        ],
    );
    assert!(add.status.success());
    let added: Value = serde_json::from_slice(&add.stdout).expect("json");
    let job_id = added["job_id"].as_str().unwrap().to_string();

    let serve = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--once",
            "--max-steps",
            "1",
            "--json",
        ],
    );
    assert!(serve.status.success());
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    let dir = PathBuf::from(processed["output_dir"].as_str().unwrap());
    let tail = Path::new(".skycot").join("outputs").join(&job_id);
    assert!(
        dir.ends_with(&tail),
        "output_dir {dir:?} should end with {tail:?}"
    );
}
