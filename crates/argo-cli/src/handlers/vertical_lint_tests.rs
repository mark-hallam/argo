//! Unit tests for `handlers::vertical_lint`. Sibling-mod
//! pattern keeps the production module under the 500-LOC budget.

use super::*;
use argo_vertical::LoadedVertical;

fn write_vertical(dir: &std::path::Path, body: &str) -> LoadedVertical {
    std::fs::write(dir.join("vertical.toml"), body).unwrap();
    let src = std::fs::read_to_string(dir.join("vertical.toml")).unwrap();
    let manifest = argo_vertical::parse(&src).unwrap();
    LoadedVertical {
        manifest,
        root: dir.to_path_buf(),
    }
}

#[test]
fn lint_clean_when_manifest_only_declares_required_sections() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n",
    );
    let findings = lint_loaded(&v);
    assert!(findings.is_empty(), "got: {findings:?}");
}

// ───────── instruction template lint (#736) ──────────────

#[test]
fn lint_template_clean_when_only_known_keys_referenced() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"d\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [actions.verify_price]\n\
         risk = \"low\"\nrequires_approval = false\ndescription = \"x\"\nexecutor = \"human\"\n\
         instruction_template = \"action_id={{ action_id }}, target=${{ payload.target }}, \
         platform={{ platform }}, item={{ item_id }}\"\n",
    );
    let findings = lint_loaded(&v);
    let template_findings: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.code == "actions.template.unknown_key")
        .collect();
    assert!(
        template_findings.is_empty(),
        "expected no template lint findings; got: {template_findings:?}"
    );
}

#[test]
fn lint_warns_when_template_references_unknown_top_level_key() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"d\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [actions.verify_price]\n\
         risk = \"low\"\nrequires_approval = false\ndescription = \"x\"\nexecutor = \"human\"\n\
         instruction_template = \"step={{ stepz }} | id={{ action_id }}\"\n",
    );
    let findings = lint_loaded(&v);
    let f = findings
        .iter()
        .find(|f| f.code == "actions.template.unknown_key")
        .expect("expected unknown_key finding");
    assert_eq!(f.severity, Severity::Warn);
    assert!(f.message.contains("stepz"), "got: {}", f.message);
    assert!(f.message.contains("verify_price"), "got: {}", f.message);
    assert!(f.message.contains("default"), "got: {}", f.message);
}

#[test]
fn lint_warns_on_unknown_key_in_per_platform_override() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"d\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [actions.post_listing]\n\
         risk = \"medium\"\nrequires_approval = true\ndescription = \"x\"\nexecutor = \"human\"\n\
         [actions.post_listing.instruction_templates]\n\
         ebay = \"price={{ payload.tytle }}\"\n",
    );
    let findings = lint_loaded(&v);
    // payload.tytle is treated as a valid payload reference
    // (we don't enforce payload-schema match — that's a deeper
    // lint that requires the action's payload_schema). So this
    // template should be CLEAN at the lint level.
    let template_findings: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.code == "actions.template.unknown_key")
        .collect();
    assert!(
        template_findings.is_empty(),
        "payload.<x> references should not trigger lint (no payload_schema check yet); got: {template_findings:?}"
    );
}

#[test]
fn lint_warns_on_unknown_top_level_key_in_per_platform_override() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"d\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [actions.post_listing]\n\
         risk = \"medium\"\nrequires_approval = true\ndescription = \"x\"\nexecutor = \"human\"\n\
         [actions.post_listing.instruction_templates]\n\
         ebay = \"buyer={{ buyrer }}\"\n",
    );
    let findings = lint_loaded(&v);
    let f = findings
        .iter()
        .find(|f| f.code == "actions.template.unknown_key")
        .expect("expected unknown_key finding");
    assert!(f.message.contains("buyrer"), "got: {}", f.message);
    assert!(f.message.contains("ebay"), "got: {}", f.message);
}

#[test]
fn lint_clean_when_template_is_empty_or_unmatched_braces() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"d\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [actions.verify_price]\n\
         risk = \"low\"\nrequires_approval = false\ndescription = \"x\"\nexecutor = \"human\"\n\
         instruction_template = \"unmatched {{ open with no close\"\n",
    );
    let findings = lint_loaded(&v);
    let template_findings: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.code == "actions.template.unknown_key")
        .collect();
    assert!(
        template_findings.is_empty(),
        "unmatched {{ should not trip the lint; got: {template_findings:?}"
    );
}

#[test]
fn lint_warns_on_unknown_tool_in_allowed_list() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\", \"file_reed_typo\"]\ndenied = []\n",
    );
    let findings = lint_loaded(&v);
    let f = findings
        .iter()
        .find(|f| f.code == "tools.allowed.unknown_name")
        .expect("expected tools.allowed.unknown_name finding");
    assert_eq!(f.severity, Severity::Warn);
    assert!(f.message.contains("file_reed_typo"), "got: {}", f.message);
}

#[test]
fn lint_warns_on_unknown_tool_in_denied_list() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = []\ndenied = [\"bogus_tool\"]\n",
    );
    let findings = lint_loaded(&v);
    let f = findings
        .iter()
        .find(|f| f.code == "tools.denied.unknown_name")
        .expect("expected tools.denied.unknown_name finding");
    assert_eq!(f.severity, Severity::Warn);
    assert!(f.message.contains("bogus_tool"));
}

#[test]
fn lint_warns_on_orphan_escalation_tool() {
    // delegate_task in escalation but neither in allowed nor
    // denied → orphan / dead-code gate.
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\"]\ndenied = []\n\
         [escalation]\nhuman_approval_required = [\"delegate_task\"]\n",
    );
    let findings = lint_loaded(&v);
    let f = findings
        .iter()
        .find(|f| f.code == "escalation.orphan_tool")
        .expect("expected escalation.orphan_tool finding");
    assert_eq!(f.severity, Severity::Warn);
    assert!(f.message.contains("delegate_task"));
}

#[test]
fn lint_does_not_warn_when_escalation_tool_is_in_denied_list() {
    // Defence-in-depth: tool in BOTH denied and escalation. The
    // gate is technically dead code (deny wins) but the
    // operator is being safe — no warn.
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\"]\ndenied = [\"delegate_task\"]\n\
         [escalation]\nhuman_approval_required = [\"delegate_task\"]\n",
    );
    let findings = lint_loaded(&v);
    assert!(
        findings.iter().all(|f| f.code != "escalation.orphan_tool"),
        "should not warn on intentional defence-in-depth: {findings:?}"
    );
}

#[test]
fn lint_warns_on_escalation_tool_with_unknown_name() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"made_up_tool\"]\ndenied = []\n\
         [escalation]\nhuman_approval_required = [\"made_up_tool\"]\n",
    );
    let findings = lint_loaded(&v);
    // Should fire BOTH unknown_tool (typo) AND not orphan
    // (since it's in allowed). Lock both.
    assert!(
        findings.iter().any(|f| f.code == "escalation.unknown_tool"),
        "got: {findings:?}"
    );
    assert!(
        findings.iter().all(|f| f.code != "escalation.orphan_tool"),
        "tool is in allowed → not orphan"
    );
}

#[test]
fn lint_emits_info_on_empty_skill_bundle() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("skills")).unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [skills]\nbundle = \"skills/\"\n",
    );
    let findings = lint_loaded(&v);
    let f = findings
        .iter()
        .find(|f| f.code == "skills.bundle_empty")
        .expect("expected skills.bundle_empty finding");
    assert_eq!(f.severity, Severity::Info);
}

#[test]
fn lint_emits_info_on_empty_evaluation_suites() {
    let dir = tempfile::tempdir().unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [evaluation]\nsuites = []\n",
    );
    let findings = lint_loaded(&v);
    let f = findings
        .iter()
        .find(|f| f.code == "evaluation.no_suites")
        .expect("expected evaluation.no_suites finding");
    assert_eq!(f.severity, Severity::Info);
}

#[test]
fn lint_emits_info_on_memory_schema_with_zero_entries() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("memory.schema.toml"),
        "schema_version = \"1\"\n",
    )
    .unwrap();
    let v = write_vertical(
        dir.path(),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [memory]\nschema = \"memory.schema.toml\"\n",
    );
    let findings = lint_loaded(&v);
    let f = findings
        .iter()
        .find(|f| f.code == "memory.schema_no_entries")
        .expect("expected memory.schema_no_entries finding");
    assert_eq!(f.severity, Severity::Info);
}
