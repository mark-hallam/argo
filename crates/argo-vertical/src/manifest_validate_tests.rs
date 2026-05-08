use super::*;
use crate::manifest::parse;
use std::path::Path;

const MINIMAL_HEADER: &str = r#"schema_version = "1"

[vertical]
id = "demo"
name = "Demo"
version = "0.1.0"
"#;

fn parse_strict(src: &str) -> ArgoResult<VerticalManifest> {
    parse(src)
}

fn err_message(src: &str) -> String {
    let err = parse_strict(src).expect_err("expected strict-mode rejection");
    format!("{err}")
}

#[test]
fn minimal_manifest_passes_strict() {
    let m = parse_strict(MINIMAL_HEADER).expect("minimal must parse + pass strict");
    validate_strict(&m).expect("running again is idempotent");
}

#[test]
fn fixture_manifests_pass_strict() {
    // Belt-and-braces: every fixture under `verticals/` must keep
    // parsing through strict mode. Walks workspace-relative so the
    // crate-scoped test still finds them.
    let mut walked = 0_usize;
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root above the crate dir");
    let verticals_dir = workspace_root.join("verticals");
    if !verticals_dir.exists() {
        return;
    }
    let entries = std::fs::read_dir(&verticals_dir).expect("read verticals dir");
    for entry in entries.flatten() {
        let manifest_path = entry.path().join("vertical.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let src = std::fs::read_to_string(&manifest_path).expect("read fixture");
        let m = parse_strict(&src)
            .unwrap_or_else(|e| panic!("fixture {} failed strict parse: {e}", manifest_path.display()));
        validate_strict(&m).unwrap_or_else(|e| {
            panic!("fixture {} failed strict validation: {e}", manifest_path.display())
        });
        walked += 1;
    }
    assert!(walked > 0, "expected at least one fixture under verticals/");
}

#[test]
fn legacy_executor_human_passes() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.draft]\nrisk = \"low\"\nrequires_approval = false\nexecutor = \"human\"\n"
    );
    parse_strict(&src).expect("human is a known kind");
}

#[test]
fn legacy_executor_unknown_kind_fails() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.draft]\nrisk = \"low\"\nrequires_approval = false\nexecutor = \"telepathic\"\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("actions.draft.executor.kind"), "got: {msg}");
    assert!(msg.contains("unknown executor kind 'telepathic'"), "got: {msg}");
}

#[test]
fn tagged_executor_unknown_kind_fails() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.draft]\nrisk = \"low\"\nrequires_approval = false\nexecutor = {{ kind = \"voodoo\" }}\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("actions.draft.executor.kind"), "got: {msg}");
    assert!(msg.contains("unknown executor kind 'voodoo'"), "got: {msg}");
}

#[test]
fn tagged_executor_empty_kind_fails() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.draft]\nrisk = \"low\"\nrequires_approval = false\nexecutor = {{ kind = \"\" }}\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("actions.draft.executor.kind"), "got: {msg}");
    assert!(msg.contains("must not be empty"), "got: {msg}");
}

#[test]
fn mcp_requires_server_and_tool() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.send]\nrisk = \"low\"\nrequires_approval = false\nexecutor = {{ kind = \"mcp\" }}\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("actions.send.executor.server"), "got: {msg}");
    assert!(msg.contains("actions.send.executor.tool"), "got: {msg}");
}

#[test]
fn mcp_with_server_and_tool_passes() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.send]\nrisk = \"low\"\nrequires_approval = false\nexecutor = {{ kind = \"mcp\", server = \"gmail\", tool = \"gmail.send\" }}\n"
    );
    parse_strict(&src).expect("complete mcp shape passes");
}

#[test]
fn api_requires_binding() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.lookup]\nrisk = \"low\"\nrequires_approval = false\nexecutor = {{ kind = \"api\" }}\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("actions.lookup.executor.binding"), "got: {msg}");
    assert!(msg.contains("requires a non-empty 'binding' field"), "got: {msg}");
}

#[test]
fn api_with_binding_passes() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.lookup]\nrisk = \"low\"\nrequires_approval = false\nexecutor = {{ kind = \"api\", binding = \"hubspot.contacts.search\" }}\n"
    );
    parse_strict(&src).expect("api with binding passes");
}

#[test]
fn computer_use_requires_profile() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.browse]\nrisk = \"low\"\nrequires_approval = false\nexecutor = {{ kind = \"computer_use\" }}\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("actions.browse.executor.profile"), "got: {msg}");
    assert!(msg.contains("requires a non-empty 'profile' field"), "got: {msg}");
}

#[test]
fn computer_use_with_profile_passes() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.browse]\nrisk = \"low\"\nrequires_approval = false\nexecutor = {{ kind = \"computer_use\", profile = \"marketplace_browser\" }}\n"
    );
    parse_strict(&src).expect("computer_use with profile passes");
}

#[test]
fn on_complete_emit_undeclared_action_fails() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.draft]\nrisk = \"low\"\nrequires_approval = false\non_complete_emit = [\"send\"]\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("actions.draft.on_complete_emit[0]"), "got: {msg}");
    assert!(msg.contains("references undeclared action 'send'"), "got: {msg}");
}

#[test]
fn on_complete_emit_declared_action_passes() {
    let src = format!(
        "{MINIMAL_HEADER}\n[actions.draft]\nrisk = \"low\"\nrequires_approval = false\non_complete_emit = [\"send\"]\n[actions.send]\nrisk = \"medium\"\nrequires_approval = true\n"
    );
    parse_strict(&src).expect("declared follow-up passes");
}

#[test]
fn provider_default_empty_fails() {
    let src = format!("{MINIMAL_HEADER}\n[provider]\ndefault = \"\"\n");
    let msg = err_message(&src);
    assert!(msg.contains("provider.default"), "got: {msg}");
    assert!(msg.contains("must not be empty"), "got: {msg}");
}

#[test]
fn provider_fallback_empty_fails() {
    let src = format!(
        "{MINIMAL_HEADER}\n[provider]\ndefault = \"anthropic/claude-sonnet-4-5\"\nfallback = \"\"\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("provider.fallback"), "got: {msg}");
    assert!(msg.contains("must not be empty"), "got: {msg}");
}

#[test]
fn provider_default_well_formed_passes() {
    let src = format!(
        "{MINIMAL_HEADER}\n[provider]\ndefault = \"anthropic/claude-sonnet-4-5\"\nfallback = \"openai/gpt-5\"\n"
    );
    parse_strict(&src).expect("valid model ids pass");
}

#[test]
fn provider_default_bare_provider_passes_today() {
    // Some shipping fixtures (ai-receptionist, sales-follow-up) use
    // `default = "anthropic"` without a model suffix. Strict mode
    // tolerates this for now; tightening the format gate is a
    // follow-up gated on fixing those fixtures.
    let src = format!("{MINIMAL_HEADER}\n[provider]\ndefault = \"anthropic\"\n");
    parse_strict(&src).expect("bare provider name still passes today");
}

#[test]
fn multiple_errors_aggregated_into_one_message() {
    // Three independent failures: empty provider default, unknown
    // executor kind, dangling on_complete_emit reference. Strict
    // mode must fold them into a single Schema error so the operator
    // gets the full punch-list at once.
    let src = format!(
        "{MINIMAL_HEADER}\n[provider]\ndefault = \"\"\n\n[actions.first]\nrisk = \"low\"\nrequires_approval = false\nexecutor = \"telepathic\"\non_complete_emit = [\"missing\"]\n"
    );
    let msg = err_message(&src);
    assert!(msg.contains("provider.default"), "got: {msg}");
    assert!(msg.contains("actions.first.executor.kind"), "got: {msg}");
    assert!(msg.contains("actions.first.on_complete_emit[0]"), "got: {msg}");
    assert!(
        msg.contains("strict validation"),
        "wrapper missing — got: {msg}"
    );
}

#[test]
fn validation_error_display_includes_path_and_message() {
    let e = ValidationError {
        path: "actions.foo.executor.kind".to_string(),
        message: "boom".to_string(),
        line: None,
    };
    assert_eq!(format!("{e}"), "actions.foo.executor.kind: boom");
}

#[test]
fn validation_error_display_with_line() {
    let e = ValidationError {
        path: "actions.foo.executor.kind".to_string(),
        message: "boom".to_string(),
        line: Some(42),
    };
    assert_eq!(format!("{e}"), "actions.foo.executor.kind (line 42): boom");
}
