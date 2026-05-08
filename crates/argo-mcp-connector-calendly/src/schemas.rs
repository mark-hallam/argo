//! MCP tool schemas for Calendly.

use argo_mcp::McpToolSchema;

/// All Calendly tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_event_types(),
        list_scheduled_events(),
        get_event(),
        cancel_event(),
        get_invitee(),
    ]
}

fn list_event_types() -> McpToolSchema {
    read_tool(
        "calendly.list_event_types",
        "List Calendly event types scoped to a user URI.",
        scoped_list_props(),
        vec!["user_uri"],
    )
}

fn list_scheduled_events() -> McpToolSchema {
    read_tool(
        "calendly.list_scheduled_events",
        "List Calendly scheduled events scoped to a user URI.",
        scheduled_list_props(),
        vec!["user_uri"],
    )
}

fn get_event() -> McpToolSchema {
    read_tool(
        "calendly.get_event",
        "Fetch one Calendly scheduled event.",
        event_ref_props(),
        Vec::new(),
    )
}

fn cancel_event() -> McpToolSchema {
    let mut props = event_ref_map();
    props.insert("reason".into(), serde_json::json!({"type": "string"}));
    write_tool(
        "calendly.cancel_event",
        "Cancel a Calendly scheduled event. Requires operator approval.",
        serde_json::Value::Object(props),
        Vec::new(),
    )
}

fn get_invitee() -> McpToolSchema {
    let mut props = event_ref_map();
    props.insert("invitee_uri".into(), serde_json::json!({"type": "string"}));
    props.insert("invitee_id".into(), serde_json::json!({"type": "string"}));
    read_tool(
        "calendly.get_invitee",
        "Fetch one Calendly event invitee.",
        serde_json::Value::Object(props),
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

fn scoped_list_props() -> serde_json::Value {
    let mut props = paging_map();
    props.insert("user_uri".into(), serde_json::json!({"type": "string"}));
    props.insert("active".into(), serde_json::json!({"type": "boolean"}));
    serde_json::Value::Object(props)
}

fn scheduled_list_props() -> serde_json::Value {
    let mut props = paging_map();
    props.insert("user_uri".into(), serde_json::json!({"type": "string"}));
    props.insert("status".into(), serde_json::json!({"type": "string"}));
    props.insert(
        "min_start_time".into(),
        serde_json::json!({"type": "string"}),
    );
    props.insert(
        "max_start_time".into(),
        serde_json::json!({"type": "string"}),
    );
    serde_json::Value::Object(props)
}

fn event_ref_props() -> serde_json::Value {
    serde_json::Value::Object(event_ref_map())
}

fn event_ref_map() -> serde_json::Map<String, serde_json::Value> {
    let mut props = serde_json::Map::new();
    props.insert("event_uri".into(), serde_json::json!({"type": "string"}));
    props.insert("event_id".into(), serde_json::json!({"type": "string"}));
    props
}

fn paging_map() -> serde_json::Map<String, serde_json::Value> {
    let mut props = serde_json::Map::new();
    props.insert(
        "count".into(),
        serde_json::json!({"type": "integer", "minimum": 1, "maximum": 100, "default": 25}),
    );
    props.insert("page_token".into(), serde_json::json!({"type": "string"}));
    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_cancel_event_requires_approval() {
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, tool.name == "calendly.cancel_event");
        }
    }
}
