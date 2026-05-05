//! End-to-end tests for Worker v2 multi-step execution (#722).
//!
//! Mock-mode only — no API key required.
//!
//! Coverage:
//!   - default `argo work serve` with the gtm-engineer manifest
//!     (which denies stub web tools) fails the pre-flight check
//!     and writes a clear `research_tools_unavailable` artifact
//!     bundle without dispatching the LLM
//!   - `argo work serve --max-steps 1` bypasses the pre-flight
//!     and runs the single-step mock path; mock now emits the
//!     four labelled sections + `[STOP: TASK_COMPLETE]` so the
//!     splitter materialises research/pricing/listings/checklist
//!   - completed_jobs body carries `steps_completed` and
//!     `stop_reason` (the v2 outcome shape)
//!   - artifact bundle includes the four section files (v1 set
//!     of transcript/assistant_output/job/result still present)

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

/// Build a tempdir-backed verticals directory that contains a
/// single fixture vertical denying both web tools. Lets the
/// pre-flight test exercise the `research_tools_unavailable`
/// path even after #723 made the production gtm-engineer
/// manifest allow them again.
fn build_no_research_verticals_dir() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("verticals tempdir");
    let v_dir = root.path().join("no-research");
    std::fs::create_dir_all(v_dir.join("prompts")).expect("prompts dir");
    std::fs::write(
        v_dir.join("prompts").join("system.md"),
        "Test vertical that denies web research tools so the worker pre-flight fires.\n",
    )
    .unwrap();
    std::fs::write(
        v_dir.join("prompts").join("persona.md"),
        "Test persona for the no-research fixture vertical.\n",
    )
    .unwrap();
    std::fs::write(
        v_dir.join("vertical.toml"),
        r#"schema_version = "1"

[vertical]
id          = "no-research"
name        = "No Research Test"
description = "Test fixture: denies web tools so worker v2 pre-flight fires."
version     = "0.0.1"
authors     = ["test"]
tags        = ["test", "draft-only"]

[role]
system_prompt = "prompts/system.md"
persona       = "prompts/persona.md"

[provider]
default   = "mock"
fallback  = "mock"
cost_tier = "balanced"

[tools]
allowed = []
denied  = ["web_search", "web_extract"]
"#,
    )
    .unwrap();
    root
}

/// Run `argo` with `ARGO_VERTICALS_DIR` overridden to a custom
/// directory (for the no-research fixture). Mirrors run_argo
/// otherwise.
fn run_argo_with_verticals(
    home: &Path,
    work: &Path,
    verticals: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .current_dir(work)
        .output()
        .expect("spawn argo")
}

/// The headline v2 + #723 acceptance test: with a vertical that
/// denies both web tools, the default multi-step worker run
/// must fail the pre-flight rather than silently produce an
/// unverified output. The production gtm-engineer manifest now
/// has the real tools in its allow list, so we use a tempdir
/// fixture vertical that restores the deny shape #722 originally
/// tested.
#[test]
fn multi_step_fails_fast_when_research_tools_unavailable() {
    let verticals = build_no_research_verticals_dir();

    // Bootstrap a workspace + ARGO_HOME pointing at the fixture
    // verticals dir so `validate_vertical` and `add` both see
    // `no-research` instead of the production set.
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = run_argo_with_verticals(
        home.path(),
        work.path(),
        verticals.path(),
        &["workspace", "init"],
    );
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let add = run_argo_with_verticals(
        home.path(),
        work.path(),
        verticals.path(),
        &[
            "work",
            "add",
            "--vertical",
            "no-research",
            "--prompt",
            "Sell these skates.",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let added: Value = serde_json::from_slice(&add.stdout).expect("json");
    let job_id = added["job_id"].as_str().expect("job_id").to_string();

    // No --max-steps → defaults to 5 → pre-flight fires.
    let serve = run_argo_with_verticals(
        home.path(),
        work.path(),
        verticals.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "no-research",
            "--once",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "work serve must succeed even when pre-flight stops the job (so the dedup set converges); stderr: {}",
        String::from_utf8_lossy(&serve.stderr),
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    assert_eq!(processed["ok"], false, "ok=false signals pre-flight stop");
    assert_eq!(processed["stop_reason"], "research_tools_unavailable");
    assert_eq!(processed["steps_completed"], 0);
    let detail = processed["stop_detail"].as_str().expect("stop_detail");
    assert!(
        detail.contains("web_search") && detail.contains("web_extract"),
        "detail must name both denied tools; got: {detail}"
    );

    // completed_jobs row exists (so the queue dedup converges).
    let by_body = run_argo_with_verticals(
        home.path(),
        work.path(),
        verticals.path(),
        &[
            "memory",
            "list",
            "--vertical",
            "no-research",
            "--body-contains",
            "completed_jobs",
            "--json",
        ],
    );
    assert!(by_body.status.success());
    let rows: Vec<Value> = String::from_utf8_lossy(&by_body.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    assert!(!rows.is_empty(), "completed_jobs row must land");

    // Artifact directory still landed — operators can still see
    // WHY the job stopped.
    let output_dir_str = processed["output_dir"]
        .as_str()
        .expect("output_dir present even on pre-flight stop");
    let output_dir = PathBuf::from(output_dir_str);
    assert!(output_dir.exists());
    let approval = std::fs::read_to_string(output_dir.join("approval_checklist.md"))
        .expect("approval_checklist.md must exist");
    assert!(
        approval.contains("stop_reason: `research_tools_unavailable`"),
        "approval checklist must surface the stop reason; got:\n{approval}"
    );
    let _ = job_id; // silence dead-code if removed later
}

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
