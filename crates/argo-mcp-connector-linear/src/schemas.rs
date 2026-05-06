//! MCP tool schemas for Linear.
use argo_mcp::McpToolSchema;

/// All Linear tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_teams(),
        list_issues(),
        get_issue(),
        create_issue(),
        update_issue(),
        add_comment(),
        list_workflows(),
    ]
}

fn list_teams() -> McpToolSchema {
    read_tool(
        "linear.list_teams",
        "List Linear teams visible to the API key.",
        paging_props(),
        Vec::new(),
    )
}

fn list_issues() -> McpToolSchema {
    read_tool(
        "linear.list_issues",
        "List Linear issues. Passes GraphQL filter through unchanged.",
        list_issue_props(),
        Vec::new(),
    )
}

fn get_issue() -> McpToolSchema {
    read_tool(
        "linear.get_issue",
        "Fetch one Linear issue by UUID or identifier.",
        serde_json::json!({"issue_id": {"type": "string"}}),
        vec!["issue_id"],
    )
}

fn create_issue() -> McpToolSchema {
    write_tool(
        "linear.create_issue",
        "Create a Linear issue. Requires operator approval.",
        issue_input_props(false),
        vec!["team_id", "title"],
    )
}

fn update_issue() -> McpToolSchema {
    write_tool(
        "linear.update_issue",
        "Update a Linear issue. Requires operator approval.",
        issue_input_props(true),
        vec!["issue_id"],
    )
}

fn add_comment() -> McpToolSchema {
    write_tool(
        "linear.add_comment",
        "Add a comment to a Linear issue. Requires operator approval.",
        serde_json::json!({
            "issue_id": {"type": "string"},
            "body": {"type": "string"}
        }),
        vec!["issue_id", "body"],
    )
}

fn list_workflows() -> McpToolSchema {
    read_tool(
        "linear.list_workflows",
        "List Linear workflow states.",
        workflow_props(),
        Vec::new(),
    )
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

fn paging_props() -> serde_json::Value {
    serde_json::json!({
        "first": {"type": "integer", "minimum": 1, "maximum": 100, "default": 25},
        "after": {"type": "string"}
    })
}

fn list_issue_props() -> serde_json::Value {
    let mut props = paging_map();
    props.insert("filter".into(), serde_json::json!({"type": "object"}));
    props.insert("team_id".into(), serde_json::json!({"type": "string"}));
    props.insert("orderBy".into(), serde_json::json!({"type": "string"}));
    serde_json::Value::Object(props)
}

fn workflow_props() -> serde_json::Value {
    let mut props = paging_map();
    props.insert("team_id".into(), serde_json::json!({"type": "string"}));
    props.insert("filter".into(), serde_json::json!({"type": "object"}));
    serde_json::Value::Object(props)
}

fn issue_input_props(include_issue_id: bool) -> serde_json::Value {
    let mut props = serde_json::json!({
        "team_id": {"type": "string"},
        "title": {"type": "string"},
        "description": {"type": "string"},
        "assignee_id": {"type": "string"},
        "state_id": {"type": "string"},
        "priority": {"type": "integer", "minimum": 0, "maximum": 4},
        "label_ids": {"type": "array", "items": {"type": "string"}}
    });
    if include_issue_id {
        props["issue_id"] = serde_json::json!({"type": "string"});
    }
    props
}

fn paging_map() -> serde_json::Map<String, serde_json::Value> {
    paging_props().as_object().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = [
            "linear.create_issue",
            "linear.update_issue",
            "linear.add_comment",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }

    #[test]
    fn list_issues_exposes_graphql_filter() {
        let schema = list_issues();
        assert!(schema.input_schema["properties"]["filter"].is_object());
    }
}
