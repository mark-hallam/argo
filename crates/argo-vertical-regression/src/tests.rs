use super::*;
use std::fs;
use tempfile::tempdir;

const MIN_MANIFEST: &str = r#"
schema_version = "1"
[vertical]
id = "synthetic"
name = "Synthetic"
version = "0.1.0"
[role]
system_prompt = "prompts/system.md"
[escalation]
human_approval_required = ["send_thing"]
[actions]
[actions.draft_thing]
risk = "low"
requires_approval = false
[actions.send_thing]
risk = "high"
requires_approval = true
"#;

fn write_case(toml: &str, fixtures: &[&str]) -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir");
    let vroot = dir.path().join("synthetic");
    fs::create_dir_all(vroot.join("eval")).unwrap();
    fs::write(vroot.join("vertical.toml"), toml).unwrap();
    fs::write(vroot.join("eval/seed.jsonl"), fixtures.join("\n")).unwrap();
    dir
}

#[test]
fn typo_in_action_type_is_flagged() {
    let dir = write_case(
        MIN_MANIFEST,
        &[r#"{"id":"f1","input":"x","expected":{"action_type":"draft_thnig"}}"#],
    );
    let cases = discover(dir.path());
    assert_eq!(cases.len(), 1, "case discovered");
    let violations = check(&cases[0]);
    assert!(
        violations.iter().any(|v| matches!(
            v,
            Violation::UndeclaredActionType { action_type, .. } if action_type == "draft_thnig"
        )),
        "expected a typo violation, got {violations:#?}"
    );
}

#[test]
fn declared_action_type_passes() {
    let dir = write_case(
        MIN_MANIFEST,
        &[r#"{"id":"f1","input":"x","expected":{"action_type":"draft_thing"}}"#],
    );
    let violations = check(&discover(dir.path())[0]);
    assert!(violations.is_empty(), "expected clean, got {violations:#?}");
}

#[test]
fn action_chain_entries_are_validated() {
    let dir = write_case(
        MIN_MANIFEST,
        &[r#"{"id":"f1","input":"x","expected":{"action_chain":["draft_thing","mystery_action"]}}"#],
    );
    let violations = check(&discover(dir.path())[0]);
    assert_eq!(violations.len(), 1, "exactly the unknown chain entry trips");
    assert!(matches!(
        &violations[0],
        Violation::UndeclaredActionType { action_type, .. } if action_type == "mystery_action"
    ));
}

#[test]
fn approval_required_must_match_escalation_block() {
    // `draft_thing` is declared but NOT under human_approval_required;
    // a fixture claiming `requires_approval: true` is a mismatch.
    let dir = write_case(
        MIN_MANIFEST,
        &[r#"{"id":"f1","input":"x","expected":{"action_type":"draft_thing","requires_approval":true}}"#],
    );
    let violations = check(&discover(dir.path())[0]);
    assert!(
        violations.iter().any(|v| matches!(
            v,
            Violation::ApprovalMismatch { expected_approval: true, declared_required: false, .. }
        )),
        "expected approval-mismatch, got {violations:#?}"
    );
}

#[test]
fn approval_auto_must_match_escalation_block() {
    // `send_thing` IS under human_approval_required; a fixture
    // claiming `requires_approval: false` is the other direction
    // of the same mismatch.
    let dir = write_case(
        MIN_MANIFEST,
        &[r#"{"id":"f1","input":"x","expected":{"action_type":"send_thing","requires_approval":false}}"#],
    );
    let violations = check(&discover(dir.path())[0]);
    assert!(
        violations.iter().any(|v| matches!(
            v,
            Violation::ApprovalMismatch { expected_approval: false, declared_required: true, .. }
        )),
        "expected approval-mismatch, got {violations:#?}"
    );
}

#[test]
fn must_include_schema_is_out_of_scope() {
    // Fixtures using the legacy must_include/must_not_include
    // regression schema have no `action_type` / `action_chain`
    // and must be skipped — not coerced into a violation.
    let dir = write_case(
        MIN_MANIFEST,
        &[r#"{"id":"f1","input":"x","expected":{"must_include":["foo"],"must_not_include":["bar"]}}"#],
    );
    let violations = check(&discover(dir.path())[0]);
    assert!(violations.is_empty(), "must_include fixtures stay silent");
}

#[test]
fn library_tier_verticals_are_skipped() {
    let toml = r#"
schema_version = "1"
[vertical]
id = "library-only"
name = "Library"
version = "0.1.0"
"#;
    let dir = tempdir().unwrap();
    let vroot = dir.path().join("library-only");
    fs::create_dir_all(&vroot).unwrap();
    fs::write(vroot.join("vertical.toml"), toml).unwrap();
    let cases = discover(dir.path());
    assert!(cases.is_empty(), "library-tier verticals must be skipped");
}

#[test]
fn verticals_without_actions_block_are_silent() {
    let toml = r#"
schema_version = "1"
[vertical]
id = "no-actions"
name = "NoActions"
version = "0.1.0"
[role]
system_prompt = "prompts/system.md"
"#;
    let dir = write_case(
        toml,
        &[r#"{"id":"f1","input":"x","expected":{"action_type":"anything"}}"#],
    );
    let cases = discover(dir.path());
    assert_eq!(cases.len(), 1);
    assert!(check(&cases[0]).is_empty(), "no [actions] = nothing to check");
}

#[test]
fn smoke_only_fixtures_are_skipped() {
    let dir = write_case(
        MIN_MANIFEST,
        &[
            r#"{"id":"f1","input":"x","expected":{"action_type":"unknown_one","expected_executor":"human"}}"#,
            r#"{"id":"f2","input":"x","expected":{"action_type":"unknown_two","requires_e2b":true}}"#,
        ],
    );
    let violations = check(&discover(dir.path())[0]);
    assert!(violations.is_empty(), "smoke-only fixtures stay silent");
}

#[test]
fn partition_separates_known_drift_from_new() {
    let mk_violation = |v: &str, a: &str| Violation::UndeclaredActionType {
        vertical: v.to_string(),
        fixture: "fx".to_string(),
        action_type: a.to_string(),
    };
    // First KNOWN_DRIFT entry — pinned, not a new violation.
    let pinned = mk_violation(KNOWN_DRIFT[0].0, KNOWN_DRIFT[0].1);
    let novel = mk_violation("brand-new-vertical", "brand_new_action");

    let (new, retired) = partition(&[pinned, novel.clone()]);
    assert_eq!(new, vec![novel], "only the unpinned violation reports");
    assert_eq!(
        retired.len(),
        KNOWN_DRIFT.len() - 1,
        "every pinned drift NOT observed reports as retired"
    );
}

#[test]
fn partition_flags_retired_baseline_when_drift_resolves() {
    // Observe nothing — every KNOWN_DRIFT row should report as
    // retired so the operator removes the stale baseline rows
    // alongside the vertical-side fix.
    let (new, retired) = partition(&[]);
    assert!(new.is_empty());
    assert_eq!(retired.len(), KNOWN_DRIFT.len());
}
