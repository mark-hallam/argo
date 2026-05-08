//! MCP tool schemas for Google Meet.

use argo_mcp::McpToolSchema;

/// All Google Meet tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_meetings(),
        schedule_meeting(),
        get_meeting(),
        list_participants(),
        get_recording(),
    ]
}

fn list_meetings() -> McpToolSchema {
    read_tool(
        "google_meet.list_meetings",
        "List Google Meet conference records.",
        meeting_list_props(),
        Vec::new(),
    )
}

fn schedule_meeting() -> McpToolSchema {
    write_tool(
        "google_meet.schedule_meeting",
        "Create a Google Meet space/link. Requires operator approval.",
        serde_json::json!({"space": {"type": "object"}}),
        Vec::new(),
    )
}

fn get_meeting() -> McpToolSchema {
    read_tool(
        "google_meet.get_meeting",
        "Fetch one Google Meet conference record.",
        serde_json::json!({"meeting_id": {"type": "string"}}),
        vec!["meeting_id"],
    )
}

fn list_participants() -> McpToolSchema {
    let mut props = paging_map();
    props.insert("meeting_id".into(), serde_json::json!({"type": "string"}));
    read_tool(
        "google_meet.list_participants",
        "List participants for a Google Meet conference record.",
        serde_json::Value::Object(props),
        vec!["meeting_id"],
    )
}

fn get_recording() -> McpToolSchema {
    read_tool(
        "google_meet.get_recording",
        "Fetch one Google Meet recording.",
        serde_json::json!({
            "meeting_id": {"type": "string"},
            "recording_id": {"type": "string"}
        }),
        vec!["meeting_id", "recording_id"],
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
        "page_size": {"type": "integer", "minimum": 1, "maximum": 100, "default": 25},
        "pageToken": {"type": "string"}
    })
}

fn meeting_list_props() -> serde_json::Value {
    let mut props = paging_map();
    props.insert("filter".into(), serde_json::json!({"type": "string"}));
    serde_json::Value::Object(props)
}

fn paging_map() -> serde_json::Map<String, serde_json::Value> {
    paging_props().as_object().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_schedule_requires_approval() {
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, tool.name == "google_meet.schedule_meeting");
        }
    }
}
