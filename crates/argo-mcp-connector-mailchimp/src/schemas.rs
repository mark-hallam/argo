//! MCP tool schemas for Mailchimp.

use argo_mcp::McpToolSchema;

/// All Mailchimp tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_audiences(),
        list_campaigns(),
        get_campaign(),
        send_campaign(),
        list_subscribers(),
        add_subscriber(),
    ]
}

fn list_audiences() -> McpToolSchema {
    read_tool(
        "mailchimp.list_audiences",
        "List Mailchimp audiences.",
        paging_props(),
        Vec::new(),
    )
}

fn list_campaigns() -> McpToolSchema {
    let mut props = paging_map();
    props.insert("type".into(), serde_json::json!({"type": "string"}));
    props.insert("status".into(), serde_json::json!({"type": "string"}));
    read_tool(
        "mailchimp.list_campaigns",
        "List Mailchimp campaigns.",
        serde_json::Value::Object(props),
        Vec::new(),
    )
}

fn get_campaign() -> McpToolSchema {
    read_tool(
        "mailchimp.get_campaign",
        "Fetch one Mailchimp campaign.",
        serde_json::json!({"campaign_id": {"type": "string"}}),
        vec!["campaign_id"],
    )
}

fn send_campaign() -> McpToolSchema {
    write_tool(
        "mailchimp.send_campaign",
        "Send a Mailchimp campaign. Requires operator approval.",
        serde_json::json!({"campaign_id": {"type": "string"}}),
        vec!["campaign_id"],
    )
}

fn list_subscribers() -> McpToolSchema {
    let mut props = paging_map();
    props.insert("list_id".into(), serde_json::json!({"type": "string"}));
    props.insert("status".into(), serde_json::json!({"type": "string"}));
    read_tool(
        "mailchimp.list_subscribers",
        "List Mailchimp audience subscribers.",
        serde_json::Value::Object(props),
        vec!["list_id"],
    )
}

fn add_subscriber() -> McpToolSchema {
    read_tool(
        "mailchimp.add_subscriber",
        "Add a Mailchimp subscriber. Autonomous-safe per connector policy.",
        serde_json::json!({
            "list_id": {"type": "string"},
            "email": {"type": "string"},
            "status": {"type": "string"},
            "merge_fields": {"type": "object"}
        }),
        vec!["list_id", "email"],
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
        "count": {"type": "integer", "minimum": 1, "maximum": 100, "default": 25},
        "offset": {"type": "integer", "minimum": 0}
    })
}

fn paging_map() -> serde_json::Map<String, serde_json::Value> {
    paging_props().as_object().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_send_campaign_requires_approval() {
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, tool.name == "mailchimp.send_campaign");
        }
    }
}
