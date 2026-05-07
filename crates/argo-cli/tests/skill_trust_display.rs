//! End-to-end coverage for the `TrustGrade` surface added to
//! `argo skill list` and `argo skill view`.
//!
//! `argo skill create` writes provenance.source=`cli-create`, which
//! `TrustGrade::from_source` does not recognise — so it falls to
//! `LearnedUnreviewed`. That property doubles as the "unknown grade
//! fallback" test: the safest default is the strictest rung.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

const SKILL_NAME: &str = "trust-display-fixture";
const BODY: &str = "Body for the trust-display fixture skill.";

fn argo(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn create_fixture_skill(home: &Path, cwd: &Path) {
    let create = argo(
        home,
        cwd,
        &[
            "skill",
            "create",
            SKILL_NAME,
            "--description",
            "Trust-display fixture",
            "--body",
            BODY,
        ],
    );
    assert!(create.status.success(), "create failed: {}", stderr(&create));
}

#[test]
fn list_json_emits_trust_grade_field_for_unknown_source() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create_fixture_skill(home.path(), work.path());

    let list = argo(
        home.path(),
        work.path(),
        &["skill", "list", "--json"],
    );
    assert!(list.status.success(), "list --json failed: {}", stderr(&list));

    let mut found = false;
    for line in stdout(&list).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(trimmed)
            .unwrap_or_else(|e| panic!("not JSON: {trimmed:?} ({e})"));
        if v["name"] == SKILL_NAME {
            // cli-create is not in TrustGrade::from_source's known
            // prefixes; the fallback is LearnedUnreviewed (strictest
            // gate is the safest default).
            assert_eq!(v["trust_grade"], "learned_unreviewed");
            found = true;
        }
    }
    assert!(found, "fixture skill missing from JSON output");
}

#[test]
fn list_markdown_includes_trust_column_and_value() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create_fixture_skill(home.path(), work.path());

    let list = argo(
        home.path(),
        work.path(),
        &["skill", "list", "--format", "markdown"],
    );
    assert!(list.status.success(), "list md failed: {}", stderr(&list));

    let out = stdout(&list);
    assert!(
        out.contains("| trust |"),
        "markdown header missing trust column:\n{out}"
    );
    assert!(
        out.contains("learned_unreviewed"),
        "markdown row missing trust value:\n{out}"
    );
}

#[test]
fn view_markdown_emits_trust_grade_line() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create_fixture_skill(home.path(), work.path());

    let view = argo(
        home.path(),
        work.path(),
        &["skill", "view", SKILL_NAME],
    );
    assert!(view.status.success(), "view failed: {}", stderr(&view));

    let out = stdout(&view);
    assert!(
        out.contains("trust grade: learned_unreviewed"),
        "view output missing trust grade line:\n{out}"
    );
}

#[test]
fn list_sort_trust_is_accepted_and_succeeds() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create_fixture_skill(home.path(), work.path());

    let list = argo(
        home.path(),
        work.path(),
        &["skill", "list", "--sort-by", "trust", "--json"],
    );
    assert!(
        list.status.success(),
        "--sort-by trust must succeed: {}",
        stderr(&list)
    );

    let mut saw_fixture = false;
    for line in stdout(&list).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(trimmed).unwrap();
        if v["name"] == SKILL_NAME {
            saw_fixture = true;
        }
    }
    assert!(saw_fixture, "trust-sorted listing dropped the fixture row");
}

#[test]
fn list_sort_unknown_value_errors_with_trust_in_help_string() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create_fixture_skill(home.path(), work.path());

    let list = argo(
        home.path(),
        work.path(),
        &["skill", "list", "--sort-by", "totally-bogus"],
    );
    assert!(
        !list.status.success(),
        "expected failure on unknown --sort-by"
    );
    let err = stderr(&list);
    assert!(
        err.contains("trust"),
        "error must list `trust` as an option:\n{err}"
    );
}

#[test]
fn view_json_includes_trust_grade_field() {
    // 6th test — the brief asks for "5+", and pairing the JSON view
    // assertion alongside the markdown one matches the parity the
    // list/view surfaces already maintain (json + markdown both).
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    create_fixture_skill(home.path(), work.path());

    let view = argo(
        home.path(),
        work.path(),
        &["skill", "view", SKILL_NAME, "--json"],
    );
    assert!(view.status.success(), "view --json failed: {}", stderr(&view));

    let v: Value = serde_json::from_slice(&view.stdout).unwrap();
    assert_eq!(v["trust_grade"], "learned_unreviewed");
}
