use super::*;

const MINIMAL: &str = r#"
schema_version = "1"

[vertical]
id = "demo"
name = "Demo"
version = "0.1.0"
"#;

const FULL: &str = r#"
schema_version = "1"

[vertical]
id = "demo"
name = "Demo Vertical"
description = "A test fixture"
version = "0.1.0"
authors = ["Test <test@example.com>"]
tags = ["test"]

[role]
system_prompt = "prompts/system.md"
persona       = "prompts/persona.md"

[provider]
default   = "anthropic/claude-sonnet-4-5"
fallback  = "openai/gpt-5"
cost_tier = "premium"

[tools]
allowed = ["web_fetch"]
denied  = ["exec"]

[memory]
schema = "memory.schema.toml"
seed   = "memory/seed/"

[skills]
bundle = "skills/"

[workflows]
files = ["workflows/main.workflow.toml"]

[escalation]
human_approval_required = ["github.comment"]
escalate_after_failures = 2
escalate_to = "humans@example.com"

[success_metrics]
deflection_target_pct       = 80
median_response_seconds     = 30
csat_threshold              = 4.5
required_audit_chain_intact = true

[evaluation]
suites = ["eval/intent.jsonl"]

[gateway]
public_endpoints = ["/v1/demo/check"]
auth_required    = true

[tui]
overlay_label   = "Demo"
default_filters = { provider = "anthropic" }
"#;

#[test]
fn parses_minimal_manifest() {
    let m = parse(MINIMAL).expect("minimal parses");
    assert_eq!(m.schema_version, "1");
    assert_eq!(m.vertical.id, "demo");
    assert_eq!(m.vertical.name, "Demo");
    assert!(m.role.is_none());
    assert!(m.provider.is_none());
}

#[test]
fn parses_full_manifest() {
    let m = parse(FULL).expect("full parses");
    let role = m.role.as_ref().expect("role present");
    assert_eq!(role.system_prompt, "prompts/system.md");
    let prov = m.provider.as_ref().expect("provider present");
    assert_eq!(prov.cost_tier, "premium");
    let tools = m.tools.as_ref().expect("tools present");
    assert_eq!(tools.allowed, vec!["web_fetch"]);
    assert_eq!(tools.denied, vec!["exec"]);
    let metrics = m.success_metrics.as_ref().expect("metrics present");
    assert_eq!(metrics.deflection_target_pct, Some(80));
    let tui = m.tui.as_ref().expect("tui present");
    assert_eq!(
        tui.default_filters.get("provider").map(String::as_str),
        Some("anthropic")
    );
}

#[test]
fn rejects_unknown_top_level_key() {
    let bad = format!("{MINIMAL}\nbogus_key = \"oops\"\n");
    let err = parse(&bad).expect_err("unknown key should fail");
    let msg = format!("{err}");
    assert!(msg.contains("vertical manifest"), "got: {msg}");
}

#[test]
fn rejects_wrong_schema_version() {
    let bad = MINIMAL.replace("\"1\"", "\"99\"");
    let err = parse(&bad).expect_err("future schema_version should fail");
    let msg = format!("{err}");
    assert!(msg.contains("schema_version '99'"), "got: {msg}");
    assert!(msg.contains("expected '1'"), "got: {msg}");
}

#[test]
fn rejects_missing_required_fields() {
    // No [vertical] section at all.
    let err = parse("schema_version = \"1\"\n").expect_err("must fail");
    let msg = format!("{err}");
    assert!(msg.contains("vertical"), "got: {msg}");
}

#[test]
fn rejects_missing_vertical_id() {
    let src = "schema_version = \"1\"\n[vertical]\nname=\"x\"\nversion=\"0.1.0\"\n";
    let err = parse(src).expect_err("missing id must fail");
    let msg = format!("{err}");
    assert!(msg.contains("id") || msg.contains("missing"), "got: {msg}");
}

#[test]
fn rejects_missing_vertical_name() {
    let src = "schema_version = \"1\"\n[vertical]\nid=\"demo\"\nversion=\"0.1.0\"\n";
    let err = parse(src).expect_err("missing name must fail");
    let msg = format!("{err}");
    assert!(msg.contains("name") || msg.contains("missing"), "got: {msg}");
}

#[test]
fn rejects_missing_vertical_version() {
    let src = "schema_version = \"1\"\n[vertical]\nid=\"demo\"\nname=\"D\"\n";
    let err = parse(src).expect_err("missing version must fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("version") || msg.contains("missing"),
        "got: {msg}"
    );
}

#[test]
fn rejects_missing_schema_version_field() {
    let src = "[vertical]\nid=\"demo\"\nname=\"D\"\nversion=\"0.1.0\"\n";
    let err = parse(src).expect_err("missing schema_version must fail");
    let msg = format!("{err}");
    assert!(msg.contains("schema_version"), "got: {msg}");
}

#[test]
fn rejects_empty_schema_version_string() {
    let src = "schema_version=\"\"\n[vertical]\nid=\"demo\"\nname=\"D\"\nversion=\"0.1.0\"\n";
    let err = parse(src).expect_err("empty schema_version must fail");
    let msg = format!("{err}");
    assert!(msg.contains("schema_version"), "got: {msg}");
}

#[test]
fn rejects_unknown_key_inside_vertical_section() {
    let src = "schema_version=\"1\"\n[vertical]\nid=\"demo\"\nname=\"D\"\nversion=\"0.1.0\"\nmystery=\"oops\"\n";
    let err = parse(src).expect_err("unknown subsection key must fail");
    let msg = format!("{err}");
    assert!(msg.contains("mystery") || msg.contains("unknown"), "got: {msg}");
}

#[test]
fn rejects_unknown_key_inside_role_section() {
    let src = "schema_version=\"1\"\n[vertical]\nid=\"demo\"\nname=\"D\"\nversion=\"0.1.0\"\n[role]\nsystem_prompt=\"prompts/system.md\"\nbogus=\"oops\"\n";
    let err = parse(src).expect_err("unknown role-section key must fail");
    let msg = format!("{err}");
    assert!(msg.contains("bogus") || msg.contains("unknown"), "got: {msg}");
}

#[test]
fn empty_workflows_files_list_parses_clean() {
    // `[workflows]` block with an empty `files = []` list is
    // structurally valid and must not be confused with "missing
    // workflows section." Pins the additive-by-default shape.
    let src = "schema_version=\"1\"\n[vertical]\nid=\"demo\"\nname=\"D\"\nversion=\"0.1.0\"\n[workflows]\nfiles=[]\n";
    let m = parse(src).expect("empty workflows.files is OK");
    assert!(m.workflows.as_ref().unwrap().files.is_empty());
}

#[test]
fn cost_tier_defaults_to_balanced() {
    let m = parse(
        r#"
schema_version = "1"

[vertical]
id = "demo"
name = "Demo"
version = "0.1.0"

[provider]
default = "anthropic/claude-sonnet-4-5"
"#,
    )
    .expect("parses");
    let prov = m.provider.as_ref().expect("provider");
    assert_eq!(prov.cost_tier, "balanced");
}
