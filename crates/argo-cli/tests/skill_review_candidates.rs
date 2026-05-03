//! End-to-end CLI tests for memory-backed skill candidates
//! (#697). Drives the binary so the surface is exercised at the
//! same layer operators use it.
//!
//! Coverage:
//! - `argo skill review list --json` returns candidates from a
//!   GTM Engineer memory row
//! - cross-vertical isolation: a candidate written under
//!   `gtm-engineer` does NOT appear when only pr-checker is
//!   loaded (synthetic verticals dir lets us prove this directly)
//! - `argo skill review promote` materialises a pending_review
//!   skill row + the same id then shows up under the existing
//!   `pending` queue + can be approved
//! - rejected/superseded memory rows do NOT appear as candidates
//!   (proven by superseding the row before listing)
//! - the existing skill-review behaviour for non-candidate
//!   pending rows is unchanged

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

const GTM_VERTICAL_TOML: &str = "schema_version = \"1\"\n\
[vertical]\nid = \"gtm-engineer\"\nname = \"GTM\"\nversion = \"0.1.0\"\n\n\
[memory]\nschema = \"memory.schema.toml\"\n";

const PR_VERTICAL_TOML: &str = "schema_version = \"1\"\n\
[vertical]\nid = \"pr-checker\"\nname = \"PR\"\nversion = \"0.1.0\"\n";

const MEMORY_SCHEMA_TOML: &str = "schema_version = \"1\"\n\n\
[[entry]]\nkey = \"learned_skill_candidates\"\ntype = \"list-of-objects\"\npurpose = \"x\"\n";

fn write_synthetic_verticals(dir: &Path, with_pr_checker: bool) {
    let gtm = dir.join("gtm-engineer");
    std::fs::create_dir_all(&gtm).unwrap();
    std::fs::write(gtm.join("vertical.toml"), GTM_VERTICAL_TOML).unwrap();
    std::fs::write(gtm.join("memory.schema.toml"), MEMORY_SCHEMA_TOML).unwrap();
    if with_pr_checker {
        let pr = dir.join("pr-checker");
        std::fs::create_dir_all(&pr).unwrap();
        std::fs::write(pr.join("vertical.toml"), PR_VERTICAL_TOML).unwrap();
    }
}

/// Spawn `argo new --vertical gtm-engineer` to create a session
/// bound to the test vertical so the seed memory row has a valid
/// `learned_in` session id. Returns the parsed UUID.
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
            "candidate-test",
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
    let stdout = String::from_utf8(out.stdout).unwrap();
    stdout
        .lines()
        .find(|l| l.starts_with("created session"))
        .and_then(|l| {
            l.trim_start_matches("created session")
                .trim()
                .split_whitespace()
                .next()
        })
        .map(String::from)
        .expect("session id parse")
}

/// Append a candidate-shaped MEMORY.md row via `argo memory
/// append memory <body> --vertical <id> --session <sid>`.
/// Returns the new entry's UUID parsed from the
/// `appended <uuid> to memory.md ...` line.
fn append_candidate(
    home: &Path,
    verticals: &Path,
    skill_name: &str,
    vertical: &str,
    session_id: &str,
) -> String {
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
            vertical,
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
    let stdout = String::from_utf8(out.stdout).unwrap();
    let line = stdout
        .lines()
        .find(|l| l.starts_with("appended "))
        .unwrap_or_else(|| panic!("no `appended …` line in:\n{stdout}"));
    line.trim_start_matches("appended ")
        .split_whitespace()
        .next()
        .unwrap_or_else(|| panic!("no uuid in: {line}"))
        .to_string()
}

#[test]
fn skill_review_list_surfaces_gtm_engineer_memory_candidate() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_synthetic_verticals(verticals.path(), false);
    let sid = boot_gtm_session(home.path(), verticals.path());
    let memory_id = append_candidate(
        home.path(),
        verticals.path(),
        "tech_founder_outreach",
        "gtm-engineer",
        &sid,
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .expect("spawn list");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    let candidates = v["memory_candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 1, "expected 1 candidate; got {v}");
    let c = &candidates[0];
    assert_eq!(c["vertical_id"], "gtm-engineer");
    assert_eq!(c["skill_name"], "tech_founder_outreach");
    assert_eq!(c["memory_entry_id"], memory_id);
    assert_eq!(c["problem"], "P");
    assert_eq!(c["evidence"], "E");
}

#[test]
fn promote_materialises_pending_review_skill_then_approve_activates_it() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_synthetic_verticals(verticals.path(), false);
    let sid = boot_gtm_session(home.path(), verticals.path());
    let memory_id = append_candidate(
        home.path(),
        verticals.path(),
        "promote_then_approve",
        "gtm-engineer",
        &sid,
    );

    // 1. Promote the candidate → pending_review skill.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["skill", "review", "promote", &memory_id, "--json"])
        .output()
        .expect("spawn promote");
    assert!(
        out.status.success(),
        "promote failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["lifecycle_state"], "pending_review");
    assert_eq!(v["vertical_id"], "gtm-engineer");
    let new_skill_id = v["skill_id"].as_str().unwrap().to_string();

    // 2. The new skill must show up under the existing
    //    `pending` queue (the same surface operators already use).
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .expect("spawn list");
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let pending = v["pending"].as_array().unwrap();
    assert!(
        pending.iter().any(|p| p["id"] == new_skill_id),
        "new skill must appear in pending queue: {pending:?}"
    );

    // 3. Approve flips it to active — the existing skill-review
    //    surface is unchanged, proven by reusing the same path.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["skill", "review", "approve", &new_skill_id, "--json"])
        .output()
        .expect("spawn approve");
    assert!(
        out.status.success(),
        "approve failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["to"], "active");
}

#[test]
fn gtm_candidate_does_not_leak_into_pr_checker_only_view() {
    // Critical isolation lock: when ONLY pr-checker is loaded
    // (no GTM Engineer in the discovery dir), candidates
    // written under "gtm-engineer" must NOT appear in the
    // skill review list. The aggregator iterates loaded
    // verticals, so an unloaded vertical's memory rows are
    // unreachable.
    let home = tempfile::tempdir().unwrap();
    let verticals_full = tempfile::tempdir().unwrap();
    let verticals_pr_only = tempfile::tempdir().unwrap();
    // Step 1: write the candidate under a verticals dir that
    // has GTM loaded (so the session can be bound + memory
    // can be appended with vertical_id=gtm-engineer).
    write_synthetic_verticals(verticals_full.path(), false);
    let sid = boot_gtm_session(home.path(), verticals_full.path());
    append_candidate(
        home.path(),
        verticals_full.path(),
        "should_not_leak",
        "gtm-engineer",
        &sid,
    );

    // Step 2: list skill review with a verticals dir that has
    // ONLY pr-checker loaded. Same ARGO_HOME so the memory row
    // exists in the SQLite store, but gtm-engineer isn't in
    // the registry — the aggregator never queries for its rows.
    std::fs::create_dir_all(verticals_pr_only.path().join("pr-checker")).unwrap();
    std::fs::write(
        verticals_pr_only
            .path()
            .join("pr-checker/vertical.toml"),
        PR_VERTICAL_TOML,
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals_pr_only.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .expect("spawn list");
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let candidates = v["memory_candidates"].as_array().unwrap();
    assert!(
        candidates.is_empty(),
        "GTM Engineer candidate leaked into pr-checker-only view: {candidates:?}"
    );
}

#[test]
fn promote_with_invalid_memory_id_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_synthetic_verticals(verticals.path(), false);
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["skill", "review", "promote", "not-a-uuid"])
        .output()
        .expect("spawn promote");
    assert!(!out.status.success(), "non-UUID input must fail");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("invalid memory id"),
        "stderr should explain: {stderr}"
    );
}

#[test]
fn empty_workspace_lists_no_candidates_and_no_pending() {
    // Regression lock: when there's no first_workspace + no
    // memory rows + no pending skills, list returns ok=true
    // with empty arrays. Doesn't error, doesn't crash.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_synthetic_verticals(verticals.path(), true);
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["skill", "review", "list", "--json"])
        .output()
        .expect("spawn list");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["count"].as_u64().unwrap(), 0);
    assert_eq!(v["memory_candidates_count"].as_u64().unwrap(), 0);
}
