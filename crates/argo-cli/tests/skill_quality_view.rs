//! Integration smoke for `argo skill quality <name>` rendering
//! modes (`claude-skill-quality-score-vis`).
//!
//! Drives the actual `argo` binary against a temp ARGO_HOME with
//! a freshly installed skill + recorded quality score. Asserts
//! markdown render carries the expected sections; JSON shape
//! matches the documented schema; --limit pulls history.

mod common;

use std::path::Path;
use std::process::{Command, Output};

fn run_argo(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(label: &str, out: &Output) {
    assert!(
        out.status.success(),
        "{label} failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn install_skill(home: &Path, name: &str) {
    let body = format!(
        "---\n\
         name: {name}\n\
         version: \"0.1.0\"\n\
         description: {name} description\n\
         provenance:\n  \
         source: learned\n  \
         hash: sha256-deadbeef\n\
         ---\n\
         body of {name}\n"
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(format!("{name}.md"));
    std::fs::write(&path, body).expect("write skill md");
    let out = run_argo(
        home,
        &["skill", "install", path.to_str().unwrap()],
    );
    assert_ok("install skill", &out);
    // Keep tempdir alive until install reads the file.
    drop(dir);
}

fn record_quality(home: &Path, name: &str) {
    // `argo skill quality recompute` is the user-visible way to
    // populate a row from real telemetry, but for the smoke we
    // need a deterministic score. Mirror the QualityRecompute CLI
    // path: pass --all to score every installed skill (the empty
    // history yields the default 100 score).
    let out = run_argo(home, &["skill", "quality-recompute", "--all"]);
    assert_ok("quality-recompute", &out);
    // Sanity: at least confirm the recompute command exits 0; the
    // existing claude-skill-quality-judge handler ensures a row
    // lands per skill.
    let _ = name;
}

#[test]
fn argo_skill_quality_text_default_renders_inputs() {
    let home = tempfile::tempdir().expect("home tempdir");
    install_skill(home.path(), "alpha");
    record_quality(home.path(), "alpha");
    let out = run_argo(home.path(), &["skill", "quality", "alpha"]);
    assert_ok("text default", &out);
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    assert!(stdout.contains("score:"), "text mode missing score: {stdout}");
    assert!(stdout.contains("inputs.success_rate:"), "text mode missing inputs: {stdout}");
}

#[test]
fn argo_skill_quality_markdown_renders_section_headings() {
    let home = tempfile::tempdir().expect("home tempdir");
    install_skill(home.path(), "beta");
    record_quality(home.path(), "beta");
    let out = run_argo(
        home.path(),
        &["skill", "quality", "beta", "--format", "markdown"],
    );
    assert_ok("markdown", &out);
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    assert!(stdout.contains("# Skill quality — `beta`"), "markdown header: {stdout}");
    assert!(stdout.contains("## Latest"), "Latest section: {stdout}");
    assert!(stdout.contains("### Inputs"), "Inputs section: {stdout}");
}

#[test]
fn argo_skill_quality_json_with_limit_includes_history_block() {
    let home = tempfile::tempdir().expect("home tempdir");
    install_skill(home.path(), "gamma");
    // Run recompute multiple times so several rows land. Each
    // recompute appends a fresh score row (no dedup on identical
    // inputs), giving us a non-empty history.
    record_quality(home.path(), "gamma");
    record_quality(home.path(), "gamma");
    record_quality(home.path(), "gamma");
    let out = run_argo(
        home.path(),
        &["skill", "quality", "gamma", "--json", "--limit", "5"],
    );
    assert_ok("json --limit", &out);
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not JSON: {e}\nstdout:\n{stdout}"));
    assert_eq!(v["skill_name"], "gamma");
    assert!(v["latest"].is_object(), "latest must be object: {stdout}");
    assert!(v["history"].is_array(), "history must be array: {stdout}");
    let history_len = v["history_len"].as_u64().expect("history_len number");
    assert!(history_len >= 1, "expected ≥1 history row given 3 recomputes; got {history_len}");
}

#[test]
fn argo_skill_quality_unknown_format_errors_cleanly() {
    let home = tempfile::tempdir().expect("home tempdir");
    install_skill(home.path(), "delta");
    record_quality(home.path(), "delta");
    let out = run_argo(
        home.path(),
        &["skill", "quality", "delta", "--format", "csv"],
    );
    assert!(!out.status.success(), "unknown format must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown --format"),
        "stderr should call out unknown format, got: {stderr}"
    );
}
