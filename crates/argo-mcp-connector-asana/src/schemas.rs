//! MCP tool schemas for Asana.
use argo_mcp::McpToolSchema;

/// All Asana tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_workspaces(),
        list_projects(),
        list_tasks(),
        get_task(),
        create_task(),
        update_task(),
        complete_task(),
        add_comment(),
    ]
}

fn list_workspaces() -> McpToolSchema {
    read_tool(
        "asana.list_workspaces",
        "List Asana workspaces.",
        serde_json::json!({}),
    )
}

fn list_projects() -> McpToolSchema {
    read_tool(
        "asana.list_projects",
        "List Asana projects by workspace or team.",
        serde_json::json!({
            "workspace": {"type": "string"},
            "team": {"type": "string"},
            "archived": {"type": "boolean"}
        }),
    )
}

fn list_tasks() -> McpToolSchema {
    read_tool(
        "asana.list_tasks",
        "List tasks in one Asana project.",
        serde_json::json!({
            "project": {"type": "string"},
            "completed_since": {"type": "string"}
        }),
    )
}

fn get_task() -> McpToolSchema {
    read_tool(
        "asana.get_task",
        "Fetch one Asana task.",
        serde_json::json!({"task_gid": {"type": "string"}}),
    )
}

fn create_task() -> McpToolSchema {
    write_tool(
        "asana.create_task",
        "Create an Asana task. Requires operator approval.",
        task_fields(false),
        vec!["name"],
    )
}

fn update_task() -> McpToolSchema {
    write_tool(
        "asana.update_task",
        "Update an Asana task. Requires operator approval.",
        task_fields(true),
        vec!["task_gid"],
    )
}

fn complete_task() -> McpToolSchema {
    write_tool(
        "asana.complete_task",
        "Complete an Asana task. Requires operator approval.",
        serde_json::json!({"task_gid": {"type": "string"}}),
        vec!["task_gid"],
    )
}

fn add_comment() -> McpToolSchema {
    write_tool(
        "asana.add_comment",
        "Add a comment to an Asana task. Requires operator approval.",
        serde_json::json!({
            "task_gid": {"type": "string"},
            "text": {"type": "string"}
        }),
        vec!["task_gid", "text"],
    )
}

fn read_tool(name: &str, description: &str, properties: serde_json::Value) -> McpToolSchema {
    let mut props = common_properties();
    merge_props(&mut props, properties);
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": false,
            "properties": props
        }),
    }
}

fn write_tool(
    name: &str,
    description: &str,
    properties: serde_json::Value,
    required: Vec<&str>,
) -> McpToolSchema {
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": true,
            "properties": properties,
            "required": required
        }),
    }
}

fn common_properties() -> serde_json::Map<String, serde_json::Value> {
    serde_json::json!({
        "limit": {"type": "integer", "minimum": 1, "maximum": 100},
        "offset": {"type": "string"},
        "opt_fields": {"type": "string"}
    })
    .as_object()
    .cloned()
    .unwrap_or_default()
}

fn task_fields(include_gid: bool) -> serde_json::Value {
    let mut props = serde_json::json!({
        "name": {"type": "string"},
        "workspace": {"type": "string"},
        "projects": {"type": "array", "items": {"type": "string"}},
        "assignee": {"type": "string"},
        "due_on": {"type": "string"},
        "notes": {"type": "string"},
        "completed": {"type": "boolean"}
    });
    if include_gid {
        props["task_gid"] = serde_json::json!({"type": "string"});
    }
    props
}

fn merge_props(props: &mut serde_json::Map<String, serde_json::Value>, extra: serde_json::Value) {
    if let Some(extra) = extra.as_object() {
        for (key, value) in extra {
            props.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = [
            "asana.create_task",
            "asana.update_task",
            "asana.complete_task",
            "asana.add_comment",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }
}
