//! MCP tool schemas for Monday.com.
use argo_mcp::McpToolSchema;

/// All Monday.com tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_boards(),
        list_items(),
        get_item(),
        create_item(),
        update_item_status(),
        add_update(),
        list_groups(),
    ]
}

fn list_boards() -> McpToolSchema {
    read_tool(
        "monday.list_boards",
        "List Monday.com boards visible to the authenticated token.",
        paging_props(),
        Vec::new(),
    )
}

fn list_items() -> McpToolSchema {
    read_tool(
        "monday.list_items",
        "List items on one Monday.com board.",
        board_item_query_props(),
        vec!["board_id"],
    )
}

fn get_item() -> McpToolSchema {
    read_tool(
        "monday.get_item",
        "Fetch one Monday.com item.",
        serde_json::json!({"item_id": {"type": "string"}}),
        vec!["item_id"],
    )
}

fn create_item() -> McpToolSchema {
    write_tool(
        "monday.create_item",
        "Create a Monday.com item. Requires operator approval.",
        create_item_props(),
        vec!["board_id", "item_name"],
    )
}

fn update_item_status() -> McpToolSchema {
    write_tool(
        "monday.update_item_status",
        "Update a Monday.com item's status column. Requires operator approval.",
        update_status_props(),
        vec!["board_id", "item_id", "status"],
    )
}

fn add_update() -> McpToolSchema {
    write_tool(
        "monday.add_update",
        "Add an update/comment to a Monday.com item. Requires operator approval.",
        serde_json::json!({
            "item_id": {"type": "string"},
            "body": {"type": "string"}
        }),
        vec!["item_id", "body"],
    )
}

fn list_groups() -> McpToolSchema {
    read_tool(
        "monday.list_groups",
        "List groups on one Monday.com board.",
        serde_json::json!({"board_id": {"type": "string"}}),
        vec!["board_id"],
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

fn board_item_query_props() -> serde_json::Value {
    let mut props = board_props();
    merge_props(&mut props, paging_props());
    serde_json::Value::Object(props)
}

fn create_item_props() -> serde_json::Value {
    let mut props = board_props();
    props.insert("item_name".into(), serde_json::json!({"type": "string"}));
    props.insert(
        "name".into(),
        serde_json::json!({"type": "string", "description": "Alias accepted for item_name."}),
    );
    props.insert("group_id".into(), serde_json::json!({"type": "string"}));
    props.insert(
        "column_values".into(),
        serde_json::json!({
            "oneOf": [{"type": "object"}, {"type": "array"}, {"type": "string"}]
        }),
    );
    serde_json::Value::Object(props)
}

fn update_status_props() -> serde_json::Value {
    serde_json::json!({
        "board_id": {"type": "string"},
        "item_id": {"type": "string"},
        "column_id": {"type": "string", "default": "status"},
        "status": {"type": "string"},
        "value": {"type": "string", "description": "Alias accepted for status."}
    })
}

fn board_props() -> serde_json::Map<String, serde_json::Value> {
    serde_json::json!({"board_id": {"type": "string"}})
        .as_object()
        .cloned()
        .unwrap_or_default()
}

fn paging_props() -> serde_json::Value {
    serde_json::json!({
        "limit": {"type": "integer", "minimum": 1, "maximum": 100},
        "page": {"type": "integer", "minimum": 1}
    })
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
            "monday.create_item",
            "monday.update_item_status",
            "monday.add_update",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }

    #[test]
    fn create_item_accepts_column_values() {
        let schema = create_item();
        assert!(schema.input_schema["properties"]["column_values"]["oneOf"].is_array());
    }
}
