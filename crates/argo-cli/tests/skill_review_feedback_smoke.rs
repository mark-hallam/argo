//! Smoke tests for `argo skill review promote --feedback-action`.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

const GTM_VERTICAL_TOML: &str = "schema_version = \"1\"\n\
[vertical]\nid = \"gtm-engineer\"\nname = \"GTM\"\nversion = \"0.1.0\"\n\n\
[memory]\nschema = \"memory.schema.toml\"\n";

const MEMORY_SCHEMA_TOML: &str = "schema_version = \"1\"\n\n\
[[entry]]\nkey = \"learned_skill_candidates\"\ntype = \"list-of-objects\"\npurpose = \"x\"\n";

struct Fixture {
    home: tempfile::TempDir,
    verticals: tempfile::TempDir,
    memory_id: String,
}

fn write_synthetic_verticals(dir: &Path) {
    let gtm = dir.join("gtm-engineer");
    std::fs::create_dir_all(&gtm).unwrap();
    std::fs::write(gtm.join("vertical.toml"), GTM_VERTICAL_TOML).unwrap();
    std::fs::write(gtm.join("memory.schema.toml"), MEMORY_SCHEMA_TOML).unwrap();
}

fn fixture(skill_name: &str) -> Fixture {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_synthetic_verticals(verticals.path());
    let sid = boot_gtm_session(home.path(), verticals.path());
    let memory_id = append_candidate(home.path(), verticals.path(), skill_name, &sid);
    Fixture {
        home,
        verticals,
        memory_id,
    }
}

fn boot_gtm_session(home: &Path, verticals: &Path) -> String {
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .args([
            "new",
            "--provider",
            "anthropic",
            "--model",
            "claude-haiku-4-5",
            "--title",
            "feedback-test",
            "--vertical",
            "gtm-engineer",
        ])
        .output()
        .expect("spawn new");
    assert!(
        out.status.success(),
        "argo new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("created session"))
        .and_then(|l| l.trim_start_matches("created session").split_whitespace().next())
        .map(str::to_string)
        .expect("session id")
}

fn append_candidate(home: &Path, verticals: &Path, skill_name: &str, session_id: &str) -> String {
    let body = format!(
        r#"{{"schema_key":"learned_skill_candidates","skill_name":"{skill_name}","problem":"P","reusable_pattern":"R","evidence":"E","expected_reuse":"X","risk":"K"}}"#
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .args([
            "memory",
            "append",
            "memory",
            &body,
            "--vertical",
            "gtm-engineer",
            "--session",
            session_id,
        ])
        .output()
        .expect("spawn memory append");
    assert!(
        out.status.success(),
        "memory append failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("appended "))
        .and_then(|l| l.trim_start_matches("appended ").split_whitespace().next())
        .map(str::to_string)
        .expect("memory id")
}

fn record_feedback(f: &Fixture, vote: &str, action_id: &str) {
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", f.home.path())
        .args(["feedback", vote, action_id, "--json"])
        .output()
        .expect("spawn feedback");
    assert!(
        out.status.success(),
        "feedback failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn promote(f: &Fixture, args: &[&str]) -> std::process::Output {
    let mut full = vec!["skill", "review", "promote", f.memory_id.as_str()];
    full.extend_from_slice(args);
    Command::new(argo_bin())
        .env("ARGO_HOME", f.home.path())
        .env("ARGO_VERTICALS_DIR", f.verticals.path())
        .args(full)
        .output()
        .expect("spawn promote")
}

fn pending_count(f: &Fixture) -> usize {
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", f.home.path())
        .env("ARGO_VERTICALS_DIR", f.verticals.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .expect("spawn list");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    v["pending"].as_array().unwrap().len()
}

#[test]
fn promote_help_shows_feedback_action_flag() {
    let out = Command::new(argo_bin())
        .args(["skill", "review", "promote", "--help"])
        .output()
        .expect("spawn help");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("--feedback-action <ACTION_ID>"), "{stdout}");
}

#[test]
fn promote_without_feedback_flag_keeps_default_behavior() {
    let f = fixture("default_feedback_optional");
    let out = promote(&f, &["--json"]);
    assert!(
        out.status.success(),
        "promote failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v.get("feedback").is_none(), "default JSON changed: {v}");
}

#[test]
fn promote_with_missing_feedback_blocks_without_creating_skill() {
    let f = fixture("missing_feedback_blocks");
    let out = promote(&f, &["--feedback-action", "missing-action", "--json"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no operator feedback found"), "{stderr}");
    assert_eq!(pending_count(&f), 0);
}

#[test]
fn promote_with_down_feedback_blocks_without_creating_skill() {
    let f = fixture("down_feedback_blocks");
    record_feedback(&f, "down", "action-down");
    let out = promote(&f, &["--feedback-action", "action-down", "--json"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("latest operator feedback"), "{stderr}");
    assert!(stderr.contains("down"), "{stderr}");
    assert_eq!(pending_count(&f), 0);
}

#[test]
fn promote_with_up_feedback_succeeds_and_reports_feedback() {
    let f = fixture("up_feedback_allows");
    record_feedback(&f, "up", "action-up");
    let out = promote(&f, &["--feedback-action", "action-up", "--json"]);
    assert!(
        out.status.success(),
        "promote failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["feedback"]["action_id"], "action-up");
    assert_eq!(v["feedback"]["vote"], "up");
}
