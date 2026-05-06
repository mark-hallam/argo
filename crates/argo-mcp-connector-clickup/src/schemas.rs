//! MCP tool schemas for ClickUp.
use argo_mcp::McpToolSchema;

/// All ClickUp tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_workspaces(),
        list_spaces(),
        list_lists(),
        list_tasks(),
        get_task(),
        create_task(),
        update_task(),
        add_comment(),
    ]
}

fn list_workspaces() -> McpToolSchema {
    read_tool(
        "clickup.list_workspaces",
        "List ClickUp workspaces visible to the token.",
        serde_json::json!({}),
        Vec::new(),
    )
}

fn list_spaces() -> McpToolSchema {
    read_tool(
        "clickup.list_spaces",
        "List spaces in one ClickUp workspace.",
        serde_json::json!({
            "workspace_id": {"type": "string"},
            "team_id": {"type": "string", "description": "Alias accepted for workspace_id."},
            "archived": {"type": "boolean"}
        }),
        vec!["workspace_id"],
    )
}

fn list_lists() -> McpToolSchema {
    read_tool(
        "clickup.list_lists",
        "List folderless lists in one ClickUp space.",
        serde_json::json!({
            "space_id": {"type": "string"},
            "archived": {"type": "boolean"}
        }),
        vec!["space_id"],
    )
}

fn list_tasks() -> McpToolSchema {
    read_tool(
        "clickup.list_tasks",
        "List tasks in one ClickUp list.",
        list_task_props(),
        vec!["list_id"],
    )
}

fn get_task() -> McpToolSchema {
    read_tool(
        "clickup.get_task",
        "Fetch one ClickUp task.",
        task_id_props(),
        vec!["task_id"],
    )
}

fn create_task() -> McpToolSchema {
    write_tool(
        "clickup.create_task",
        "Create a ClickUp task. Requires operator approval.",
        task_body_props(true),
        vec!["list_id", "name"],
    )
}

fn update_task() -> McpToolSchema {
    write_tool(
        "clickup.update_task",
        "Update a ClickUp task. Requires operator approval.",
        task_body_props(false),
        vec!["task_id"],
    )
}

fn add_comment() -> McpToolSchema {
    write_tool(
        "clickup.add_comment",
        "Add a comment to a ClickUp task. Requires operator approval.",
        comment_props(),
        vec!["task_id", "comment_text"],
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

fn list_task_props() -> serde_json::Value {
    let mut props = serde_json::json!({
        "list_id": {"type": "string"},
        "page": {"type": "integer", "minimum": 0},
        "archived": {"type": "boolean"},
        "include_closed": {"type": "boolean"},
        "include_markdown_description": {"type": "boolean"},
        "subtasks": {"type": "boolean"},
        "order_by": {"type": "string"},
        "reverse": {"type": "boolean"},
        "statuses": {"type": "array", "items": {"type": "string"}},
        "assignees": {"type": "array", "items": {"type": "string"}},
        "tags": {"type": "array", "items": {"type": "string"}}
    });
    merge_props(&mut props, date_filter_props());
    props
}

fn task_id_props() -> serde_json::Value {
    serde_json::json!({
        "task_id": {"type": "string"},
        "custom_task_ids": {"type": "boolean"},
        "team_id": {"type": "string"},
        "include_subtasks": {"type": "boolean"},
        "include_markdown_description": {"type": "boolean"}
    })
}

fn task_body_props(include_list_id: bool) -> serde_json::Value {
    let mut props = serde_json::json!({
        "task_id": {"type": "string"},
        "name": {"type": "string"},
        "description": {"type": "string"},
        "markdown_content": {"type": "string"},
        "status": {"type": "string"},
        "priority": {"type": ["integer", "null"]},
        "assignees": {"type": "array", "items": {"type": "integer"}},
        "tags": {"type": "array", "items": {"type": "string"}},
        "due_date": {"type": "integer"},
        "due_date_time": {"type": "boolean"},
        "start_date": {"type": "integer"},
        "start_date_time": {"type": "boolean"},
        "time_estimate": {"type": "integer"},
        "points": {"type": "number"},
        "notify_all": {"type": "boolean"},
        "parent": {"type": ["string", "null"]},
        "links_to": {"type": ["string", "null"]},
        "custom_task_ids": {"type": "boolean"},
        "team_id": {"type": "string"}
    });
    if include_list_id {
        props["list_id"] = serde_json::json!({"type": "string"});
        props["custom_fields"] = serde_json::json!({"type": "array"});
    }
    props
}

fn comment_props() -> serde_json::Value {
    serde_json::json!({
        "task_id": {"type": "string"},
        "comment_text": {"type": "string"},
        "text": {"type": "string", "description": "Alias accepted for comment_text."},
        "notify_all": {"type": "boolean", "default": false},
        "assignee": {"type": "integer"},
        "group_assignee": {"type": "string"},
        "custom_task_ids": {"type": "boolean"},
        "team_id": {"type": "string"}
    })
}

fn date_filter_props() -> serde_json::Value {
    serde_json::json!({
        "due_date_gt": {"type": "integer"},
        "due_date_lt": {"type": "integer"},
        "date_created_gt": {"type": "integer"},
        "date_created_lt": {"type": "integer"},
        "date_updated_gt": {"type": "integer"},
        "date_updated_lt": {"type": "integer"}
    })
}

fn merge_props(target: &mut serde_json::Value, extra: serde_json::Value) {
    if let (Some(target), Some(extra)) = (target.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = [
            "clickup.create_task",
            "clickup.update_task",
            "clickup.add_comment",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }

    #[test]
    fn list_tasks_exposes_filters() {
        let schema = list_tasks();
        assert!(schema.input_schema["properties"]["statuses"].is_object());
        assert!(schema.input_schema["properties"]["date_updated_gt"].is_object());
    }
}
