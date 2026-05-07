mod common;

use common::argo_bin;
use std::path::Path;
use std::process::{Command, Output};

/// `argo skill quality-trend <missing>` exits non-zero on a fresh
/// ARGO_HOME and surfaces the brief-required `no active skill` error.
/// Smoke-coverage of the new `Command::Skill { cmd: SkillCmd::QualityTrend
/// }` dispatch path.
#[test]
fn skill_quality_trend_missing_skill_errors_clearly() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let out = run(
        home.path(),
        work.path(),
        &["skill", "quality-trend", "ghost-skill", "--json"],
    );
    assert!(
        !out.status.success(),
        "expected failure for missing skill; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no active skill"),
        "expected 'no active skill' in stderr, got: {stderr}"
    );
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}
