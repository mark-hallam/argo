//! End-to-end tests for `argo work` output artifacts (#721).
//!
//! Drives the binary so the surface is exercised at the same
//! layer operators use it. Mock-mode only — no API key required.
//!
//! Coverage:
//!   - `argo work serve --once` writes a `.skycot/outputs/<job_id>/`
//!     directory containing transcript.md, assistant_output.md,
//!     job.json, and result.json
//!   - assistant_output.md carries the full assistant text (no
//!     2000-char preview cap, no 80-char snippet)
//!   - transcript.md contains BOTH the user prompt and the
//!     assistant body, in order
//!   - job.json + result.json carry the documented field set
//!   - the completed_jobs memory entry references output_dir

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

/// Fetch the body of every memory row matching `schema_key`
/// under the gtm-engineer namespace. Same pattern the existing
/// `tests/work_queue.rs` uses (kept inlined here to avoid a
/// shared-helper module that would itself need to live in
/// tests/common — and `common::argo_bin` is already there).
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

// ─────────────────────────────────────────────────────── tests

#[test]
fn work_serve_writes_full_output_artifact_directory() {
    let (home, work) = fresh_workspace();

    // Long-ish prompt so we can prove no preview-cap truncation
    // in the on-disk artifacts (vs. the in-DB completed_jobs
    // row which still carries a 2000-char preview).
    let prompt = format!(
        "Analyze this weak campaign: 100 emails, 2 replies, 0 positive. \
         Recommend the next GTM move. {}",
        "x".repeat(400),
    );

    let add = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            &prompt,
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
    assert_eq!(processed["job_id"], job_id);
    let session_id = processed["session_id"]
        .as_str()
        .expect("session_id")
        .to_string();

    // (a) `output_dir` is surfaced on the work-serve --json line
    let output_dir_str = processed["output_dir"]
        .as_str()
        .expect("output_dir on work serve json");
    let output_dir = PathBuf::from(output_dir_str);
    assert!(
        output_dir.exists(),
        "output dir must exist on disk: {output_dir:?}"
    );

    // The dir lives under the cwd's `.skycot/outputs/<job_id>/`
    let expected_tail = Path::new(".skycot").join("outputs").join(&job_id);
    assert!(
        output_dir.ends_with(&expected_tail),
        "output_dir {output_dir:?} should end with {expected_tail:?}"
    );

    // (b) all four artifact files are present
    let transcript_path = output_dir.join("transcript.md");
    let assistant_path = output_dir.join("assistant_output.md");
    let job_path = output_dir.join("job.json");
    let result_path = output_dir.join("result.json");
    for p in &[&transcript_path, &assistant_path, &job_path, &result_path] {
        assert!(p.exists(), "expected artifact missing: {p:?}");
    }

    // (c) assistant_output.md carries the FULL assistant text
    // (mock-mode echoes the prompt back, so we can match against it)
    let asst = std::fs::read_to_string(&assistant_path).expect("read assistant_output.md");
    assert!(
        asst.starts_with("[mock]"),
        "assistant_output.md should be mock text; got: {asst}"
    );
    assert!(
        asst.contains(&prompt),
        "assistant_output.md must contain the full prompt verbatim — no 2000-char preview cap"
    );
    // The mock payload is necessarily larger than the prompt
    // alone — so on-disk length > prompt length proves we're not
    // looking at a preview snippet.
    assert!(asst.len() > prompt.len() + 80);

    // (d) transcript.md contains both user + assistant bodies in order
    let transcript = std::fs::read_to_string(&transcript_path).expect("read transcript.md");
    assert!(transcript.contains("# Worker run transcript"));
    assert!(transcript.contains("- vertical: `gtm-engineer`"));
    assert!(transcript.contains("- provider: `mock/mock`"));
    assert!(transcript.contains("[01] user"));
    assert!(transcript.contains("[02] assistant"));
    assert!(
        transcript.contains(&prompt),
        "transcript must include the full prompt body, not a snippet"
    );
    assert!(
        transcript.contains("[mock]"),
        "transcript must include the assistant body"
    );
    // user before assistant.
    let user_pos = transcript.find("[01] user").expect("user heading");
    let asst_pos = transcript
        .find("[02] assistant")
        .expect("assistant heading");
    assert!(
        user_pos < asst_pos,
        "transcript must order user before assistant"
    );

    // (e) job.json metadata
    let job_json: Value = serde_json::from_slice(&std::fs::read(&job_path).expect("read job.json"))
        .expect("parse job.json");
    assert_eq!(job_json["job_id"], job_id);
    assert_eq!(job_json["vertical_id"], "gtm-engineer");
    assert_eq!(job_json["session_id"], session_id);
    assert_eq!(job_json["provider"], "mock");
    assert_eq!(job_json["model"], "mock");
    assert_eq!(job_json["status"], "completed");
    assert!(job_json["created_at"].as_str().is_some());
    assert!(job_json["completed_at"].as_str().is_some());

    // (f) result.json pointers
    let result_json: Value =
        serde_json::from_slice(&std::fs::read(&result_path).expect("read result.json"))
            .expect("parse result.json");
    assert!(result_json["assistant_id"].as_str().is_some());
    assert!(result_json["assistant_text_preview"].as_str().is_some());
    let preview = result_json["assistant_text_preview"].as_str().unwrap();
    assert!(preview.starts_with("[mock]"), "preview echoes mock prefix");
    let full_path = result_json["full_assistant_text_path"]
        .as_str()
        .expect("full_assistant_text_path");
    let xscript_path = result_json["transcript_path"]
        .as_str()
        .expect("transcript_path");
    assert!(full_path.ends_with("assistant_output.md"));
    assert!(xscript_path.ends_with("transcript.md"));

    // (g) completed_jobs memory entry carries output_dir
    let completed = list_jobs_by_schema(home.path(), work.path(), "completed_jobs");
    assert_eq!(
        completed.len(),
        1,
        "expected one completed_jobs row; got {completed:?}"
    );
    let dir_in_memory = completed[0]["output_dir"]
        .as_str()
        .expect("completed_jobs body must carry output_dir");
    assert_eq!(dir_in_memory, output_dir_str);
}
