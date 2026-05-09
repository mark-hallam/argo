//! End-to-end smoke for `argo genesis revise`.
//!
//! Scaffolds a real vertical in a tempdir via `argo_genesis`, then
//! exec's the built binary with `--target prompt --dry-run --json`
//! and asserts (a) the binary returns success, (b) the JSON envelope
//! reports a non-empty diff against the system prompt, (c) nothing
//! on disk actually changed.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn argo(home: &Path, cwd: &Path, verticals: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn revise_dry_run_prompt_reports_non_empty_diff_and_does_not_write() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let verticals = work.path().join("verticals");

    let plan = argo_genesis::plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
    )
    .expect("plan");
    let vertical_dir = verticals.join(&plan.vertical_id);
    argo_genesis::write_vertical(&plan, &vertical_dir).expect("write");

    let prompt_path = vertical_dir.join("prompts/system.md");
    let manifest_path = vertical_dir.join("vertical.toml");
    let eval_path = vertical_dir.join("eval-seed.toml");
    let prompt_before = std::fs::read_to_string(&prompt_path).unwrap();
    let manifest_before = std::fs::read_to_string(&manifest_path).unwrap();
    let eval_before = std::fs::read_to_string(&eval_path).unwrap();

    let result = argo(
        home.path(),
        work.path(),
        &verticals,
        &[
            "genesis",
            "revise",
            "--vertical",
            &plan.vertical_id,
            "--feedback",
            "be more terse and skip the social-media actions",
            "--target",
            "prompt",
            "--dry-run",
            "--json",
        ],
    );
    assert!(
        result.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );

    let envelope: Value = serde_json::from_slice(&result.stdout)
        .unwrap_or_else(|e| panic!("non-JSON stdout: {e}\n{}", String::from_utf8_lossy(&result.stdout)));
    assert_eq!(envelope["vertical"], plan.vertical_id);
    assert_eq!(envelope["target"], "prompt");
    assert_eq!(envelope["dry_run"], true);
    let written = envelope["written"]
        .as_array()
        .expect("written should be an array");
    assert_eq!(
        written.len(),
        1,
        "--target prompt should report exactly one pending file: {envelope}"
    );
    let changed_lines = written[0]["changed_lines"]
        .as_u64()
        .expect("changed_lines should be a number");
    assert!(
        changed_lines > 0,
        "diff should be non-empty for a non-trivial feedback string: {envelope}"
    );

    assert_eq!(
        std::fs::read_to_string(&prompt_path).unwrap(),
        prompt_before,
        "dry-run must not modify the prompt"
    );
    assert_eq!(
        std::fs::read_to_string(&manifest_path).unwrap(),
        manifest_before,
        "dry-run must not modify the manifest"
    );
    assert_eq!(
        std::fs::read_to_string(&eval_path).unwrap(),
        eval_before,
        "dry-run must not modify the eval seed"
    );
}
