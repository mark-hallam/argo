//! MCP tool schemas for GitHub Issues.

use argo_mcp::McpToolSchema;

/// All GitHub Issues tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_issues(),
        get_issue(),
        create_issue(),
        update_issue(),
        add_comment(),
        close_issue(),
        label_issue(),
    ]
}

fn list_issues() -> McpToolSchema {
    read_tool(
        "github_issues.list_issues",
        "List GitHub issues in one repository.",
        base_repo_props(serde_json::json!({
            "state": {"type": "string", "enum": ["open", "closed", "all"], "default": "open"},
            "labels": {"type": "string"},
            "since": {"type": "string"},
            "per_page": {"type": "integer", "minimum": 1, "maximum": 100, "default": 30}
        })),
        vec!["owner", "repo"],
    )
}

fn get_issue() -> McpToolSchema {
    read_tool(
        "github_issues.get_issue",
        "Fetch one GitHub issue.",
        issue_props(serde_json::json!({})),
        vec!["owner", "repo", "issue_number"],
    )
}

fn create_issue() -> McpToolSchema {
    write_tool(
        "github_issues.create_issue",
        "Create a GitHub issue. Requires operator approval.",
        base_repo_props(serde_json::json!({
            "title": {"type": "string"},
            "body": {"type": "string"},
            "labels": {"type": "array", "items": {"type": "string"}}
        })),
        vec!["owner", "repo", "title"],
    )
}

fn update_issue() -> McpToolSchema {
    write_tool(
        "github_issues.update_issue",
        "Update a GitHub issue. Requires operator approval.",
        issue_props(serde_json::json!({
            "title": {"type": "string"},
            "body": {"type": "string"},
            "state": {"type": "string", "enum": ["open", "closed"]},
            "labels": {"type": "array", "items": {"type": "string"}}
        })),
        vec!["owner", "repo", "issue_number"],
    )
}

fn add_comment() -> McpToolSchema {
    write_tool(
        "github_issues.add_comment",
        "Add a comment to a GitHub issue. Requires operator approval.",
        issue_props(serde_json::json!({"body": {"type": "string"}})),
        vec!["owner", "repo", "issue_number", "body"],
    )
}

fn close_issue() -> McpToolSchema {
    write_tool(
        "github_issues.close_issue",
        "Close a GitHub issue. Requires operator approval.",
        issue_props(serde_json::json!({})),
        vec!["owner", "repo", "issue_number"],
    )
}

fn label_issue() -> McpToolSchema {
    write_tool(
        "github_issues.label_issue",
        "Add or remove GitHub issue labels. Requires operator approval.",
        issue_props(serde_json::json!({
            "add": {"type": "array", "items": {"type": "string"}},
            "remove": {"type": "array", "items": {"type": "string"}}
        })),
        vec!["owner", "repo", "issue_number"],
    )
}

fn base_repo_props(extra: serde_json::Value) -> serde_json::Value {
    let mut props = serde_json::json!({
        "owner": {"type": "string"},
        "repo": {"type": "string"}
    });
    extend_object(&mut props, extra);
    props
}

fn issue_props(extra: serde_json::Value) -> serde_json::Value {
    let mut props = base_repo_props(serde_json::json!({
        "issue_number": {"type": "integer", "minimum": 1}
    }));
    extend_object(&mut props, extra);
    props
}

fn extend_object(props: &mut serde_json::Value, extra: serde_json::Value) {
    if let (Some(dst), Some(src)) = (props.as_object_mut(), extra.as_object()) {
        dst.extend(src.clone());
    }
}

fn read_tool(
    name: &str,
    description: &str,
    properties: serde_json::Value,
    required: Vec<&str>,
) -> McpToolSchema {
    tool(name, description, false, properties, required)
}

fn write_tool(
    name: &str,
    description: &str,
    properties: serde_json::Value,
    required: Vec<&str>,
) -> McpToolSchema {
    tool(name, description, true, properties, required)
}

fn tool(
    name: &str,
    description: &str,
    approval_required: bool,
    properties: serde_json::Value,
    required: Vec<&str>,
) -> McpToolSchema {
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": approval_required,
            "properties": properties,
            "required": required
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = [
            "github_issues.create_issue",
            "github_issues.update_issue",
            "github_issues.add_comment",
            "github_issues.close_issue",
            "github_issues.label_issue",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }
}
