//! Unit tests for `super::mcp_tools` rendering + filter logic.
//!
//! These exercise the pure functions only — no subprocess spawn,
//! no manifest I/O. The integration smoke
//! (`tests/mcp_tools_smoke.rs`) drives the full path against a
//! Python stdio fixture.
use super::*;
use argo_mcp::McpToolSchema;

fn fixture(name: &str, description: &str, schema: serde_json::Value) -> McpToolSchema {
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema: schema,
    }
}

fn two_tool_fixture() -> Vec<McpToolSchema> {
    vec![
        fixture(
            "echo",
            "Echo the input back unchanged.",
            serde_json::json!({
                "type": "object",
                "properties": {"text": {"type": "string"}}
            }),
        ),
        fixture(
            "send_email",
            "Send an outbound email.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "to": {"type": "string"},
                    "subject": {"type": "string"}
                },
                "required": ["to", "subject"]
            }),
        ),
    ]
}

#[test]
fn render_markdown_pins_two_tool_layout() {
    let tools = two_tool_fixture();
    let out = render_markdown("gmail", "support", &tools, false);
    assert!(out.starts_with("# MCP tools — `gmail` (vertical `support`)"));
    assert!(out.contains("2 tools discovered."));
    assert!(out.contains("## `echo`"));
    assert!(out.contains("Echo the input back unchanged."));
    assert!(out.contains("## `send_email`"));
    assert!(out.contains("Send an outbound email."));
    // Schema body fenced as ```json …``` blocks (one per tool).
    let fences: Vec<&str> = out.match_indices("```json").map(|(_, s)| s).collect();
    assert_eq!(fences.len(), 2, "one schema fence per tool, got: {out}");
}

#[test]
fn render_markdown_singularises_count_for_one_tool() {
    let tools = vec![two_tool_fixture().remove(0)];
    let out = render_markdown("gmail", "support", &tools, false);
    assert!(out.contains("1 tool discovered."));
    assert!(!out.contains("1 tools"));
}

#[test]
fn render_markdown_empty_drops_sections() {
    let out = render_markdown("gmail", "support", &[], false);
    assert!(out.contains("0 tools discovered."));
    assert!(!out.contains("## `"));
}

#[test]
fn render_markdown_schema_only_omits_descriptions() {
    let tools = two_tool_fixture();
    let out = render_markdown("gmail", "support", &tools, true);
    assert!(out.contains("## `echo`"));
    assert!(out.contains("```json"));
    assert!(
        !out.contains("Echo the input back unchanged."),
        "schema_only must hide descriptions, got: {out}"
    );
    assert!(
        !out.contains("Send an outbound email."),
        "schema_only must hide descriptions, got: {out}"
    );
}

#[test]
fn ndjson_line_includes_description_by_default() {
    let tool = two_tool_fixture().remove(0);
    let line = ndjson_line(&tool, false).expect("ndjson serialises");
    let v: serde_json::Value = serde_json::from_str(&line).expect("valid json");
    assert_eq!(v["name"], "echo");
    assert_eq!(v["description"], "Echo the input back unchanged.");
    assert_eq!(v["input_schema"]["type"], "object");
}

#[test]
fn ndjson_line_schema_only_drops_description() {
    let tool = two_tool_fixture().remove(0);
    let line = ndjson_line(&tool, true).expect("ndjson serialises");
    let v: serde_json::Value = serde_json::from_str(&line).expect("valid json");
    assert_eq!(v["name"], "echo");
    assert!(v.get("description").is_none(), "schema_only must drop description");
    assert!(v.get("input_schema").is_some());
}

#[test]
fn apply_filter_narrows_by_regex() {
    let tools = two_tool_fixture();
    let re = compile_filter(Some("send.*"))
        .expect("regex compiles")
        .expect("Some");
    let filtered = apply_filter(tools, Some(&re));
    let names: Vec<&str> = filtered.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec!["send_email"]);
}

#[test]
fn apply_filter_none_returns_all() {
    let tools = two_tool_fixture();
    let filtered = apply_filter(tools.clone(), None);
    assert_eq!(filtered.len(), tools.len());
}

#[test]
fn compile_filter_rejects_invalid_regex() {
    let err = compile_filter(Some("[unclosed")).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("invalid --filter regex"),
        "error must call out --filter, got: {msg}"
    );
}

#[test]
fn compile_filter_none_returns_none() {
    let out = compile_filter(None).expect("None compiles");
    assert!(out.is_none());
}
