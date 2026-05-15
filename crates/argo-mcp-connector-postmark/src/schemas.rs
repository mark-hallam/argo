//! MCP tool schemas for Postmark.

use argo_mcp::McpToolSchema;

/// All Postmark tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_servers(),
        send_email(),
        list_messages(),
        get_message(),
        list_templates(),
    ]
}

fn list_servers() -> McpToolSchema {
    read_tool(
        "postmark.list_servers",
        "Fetch the Postmark server visible to the configured server token.",
        serde_json::json!({}),
        Vec::new(),
    )
}

fn send_email() -> McpToolSchema {
    write_tool(
        "postmark.send_email",
        "Send one Postmark email. Requires operator approval.",
        serde_json::json!({
            "from": {"type": "string"},
            "to": {"type": "string"},
            "subject": {"type": "string"},
            "text_body": {"type": "string"},
            "html_body": {"type": "string"},
            "cc": {"type": "string"},
            "bcc": {"type": "string"},
            "reply_to": {"type": "string"},
            "tag": {"type": "string"},
            "track_opens": {"type": "boolean"},
            "track_links": {"type": "string"},
            "message_stream": {"type": "string"},
            "headers": {"type": "array"},
            "metadata": {"type": "object"},
            "attachments": {"type": "array"}
        }),
        vec!["from", "to", "subject"],
    )
}

fn list_messages() -> McpToolSchema {
    read_tool(
        "postmark.list_messages",
        "Search outbound Postmark messages.",
        serde_json::json!({
            "count": {"type": "integer", "minimum": 1, "maximum": 500, "default": 50},
            "offset": {"type": "integer", "minimum": 0, "default": 0},
            "recipient": {"type": "string"},
            "fromemail": {"type": "string"},
            "tag": {"type": "string"},
            "status": {"type": "string"},
            "todate": {"type": "string"},
            "fromdate": {"type": "string"},
            "subject": {"type": "string"},
            "message_stream": {"type": "string"}
        }),
        Vec::new(),
    )
}

fn get_message() -> McpToolSchema {
    read_tool(
        "postmark.get_message",
        "Fetch details for one outbound Postmark message.",
        serde_json::json!({"message_id": {"type": "string"}}),
        vec!["message_id"],
    )
}

fn list_templates() -> McpToolSchema {
    read_tool(
        "postmark.list_templates",
        "List Postmark templates.",
        serde_json::json!({
            "count": {"type": "integer", "minimum": 1, "maximum": 500, "default": 100},
            "offset": {"type": "integer", "minimum": 0, "default": 0},
            "layout_template": {"type": "string"},
            "template_type": {"type": "string"}
        }),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_send_email_is_approval_required() {
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, tool.name == "postmark.send_email");
        }
    }
}
