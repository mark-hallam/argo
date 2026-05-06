//! MCP tool schemas for Jira Cloud.

use argo_mcp::McpToolSchema;
use serde_json::{json, Value};

/// All Jira tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_issues(),
        get_issue(),
        create_issue(),
        update_issue(),
        comment_issue(),
        transition_issue(),
        list_projects(),
    ]
}

fn list_issues() -> McpToolSchema {
    tool(
        "jira.list_issues",
        "Search Jira issues with a JQL query.",
        json!({
            "jql": {"type": "string"},
            "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 20},
            "startAt": {"type": "integer", "minimum": 0},
            "fields": {"oneOf": [
                {"type": "string"},
                {"type": "array", "items": {"type": "string"}}
            ]}
        }),
        &["jql"],
        false,
    )
}

fn get_issue() -> McpToolSchema {
    tool(
        "jira.get_issue",
        "Fetch one Jira issue by key or id.",
        json!({
            "issue_key": {"type": "string"},
            "fields": {"oneOf": [
                {"type": "string"},
                {"type": "array", "items": {"type": "string"}}
            ]}
        }),
        &["issue_key"],
        false,
    )
}

fn create_issue() -> McpToolSchema {
    tool(
        "jira.create_issue",
        "Create a Jira issue from a fields object.",
        json!({"fields": {"type": "object"}}),
        &["fields"],
        true,
    )
}

fn update_issue() -> McpToolSchema {
    tool(
        "jira.update_issue",
        "Update fields or update operations on one Jira issue.",
        json!({
            "issue_key": {"type": "string"},
            "fields": {"type": "object"},
            "update": {"type": "object"}
        }),
        &["issue_key"],
        true,
    )
}

fn comment_issue() -> McpToolSchema {
    tool(
        "jira.comment_issue",
        "Add a comment to one Jira issue. String bodies are converted to Atlassian Document Format.",
        json!({
            "issue_key": {"type": "string"},
            "body": {"oneOf": [{"type": "string"}, {"type": "object"}]}
        }),
        &["issue_key", "body"],
        true,
    )
}

fn transition_issue() -> McpToolSchema {
    tool(
        "jira.transition_issue",
        "Transition one Jira issue by transition id, with optional fields or update payload.",
        json!({
            "issue_key": {"type": "string"},
            "transition_id": {"type": "string"},
            "fields": {"type": "object"},
            "update": {"type": "object"}
        }),
        &["issue_key", "transition_id"],
        true,
    )
}

fn list_projects() -> McpToolSchema {
    tool(
        "jira.list_projects",
        "List Jira projects visible to the authenticated user.",
        json!({
            "query": {"type": "string"},
            "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 20},
            "startAt": {"type": "integer", "minimum": 0}
        }),
        &[],
        false,
    )
}

fn tool(
    name: &str,
    description: &str,
    properties: Value,
    required: &[&str],
    approval_required: bool,
) -> McpToolSchema {
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
            "x-argo": {"approval_required": approval_required}
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemas_mark_approval_gate_by_method() {
        let mut marked = all()
            .into_iter()
            .map(|schema| {
                let required = schema.input_schema["x-argo"]["approval_required"]
                    .as_bool()
                    .expect("approval marker");
                (schema.name, required)
            })
            .collect::<Vec<_>>();
        marked.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            marked,
            vec![
                ("jira.comment_issue".into(), true),
                ("jira.create_issue".into(), true),
                ("jira.get_issue".into(), false),
                ("jira.list_issues".into(), false),
                ("jira.list_projects".into(), false),
                ("jira.transition_issue".into(), true),
                ("jira.update_issue".into(), true),
            ]
        );
    }
}
