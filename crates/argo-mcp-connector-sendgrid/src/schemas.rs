//! MCP tool schemas for SendGrid.

use argo_mcp::McpToolSchema;

/// All SendGrid tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_templates(),
        send_email(),
        list_contacts(),
        add_contact(),
        list_lists(),
    ]
}

fn list_templates() -> McpToolSchema {
    read_tool(
        "sendgrid.list_templates",
        "List SendGrid transactional templates.",
        serde_json::json!({
            "generations": {"type": "string", "default": "dynamic"},
            "page_size": {"type": "integer", "minimum": 1, "maximum": 200},
            "page_token": {"type": "string"}
        }),
        Vec::new(),
    )
}

fn send_email() -> McpToolSchema {
    write_tool(
        "sendgrid.send_email",
        "Send an email through SendGrid. Requires operator approval.",
        serde_json::json!({
            "from": {"type": "string"},
            "to": {"type": "array", "items": {"type": "string"}},
            "subject": {"type": "string"},
            "text": {"type": "string"},
            "html": {"type": "string"},
            "template_id": {"type": "string"},
            "dynamic_template_data": {"type": "object"},
            "categories": {"type": "array", "items": {"type": "string"}}
        }),
        vec!["from", "to"],
    )
}

fn list_contacts() -> McpToolSchema {
    read_tool(
        "sendgrid.list_contacts",
        "List SendGrid Marketing Campaigns contacts.",
        serde_json::json!({
            "page_size": {"type": "integer", "minimum": 1, "maximum": 200},
            "page_token": {"type": "string"}
        }),
        Vec::new(),
    )
}

fn add_contact() -> McpToolSchema {
    read_tool(
        "sendgrid.add_contact",
        "Add one SendGrid contact. Autonomous-safe per connector policy.",
        serde_json::json!({
            "email": {"type": "string"},
            "first_name": {"type": "string"},
            "last_name": {"type": "string"},
            "custom_fields": {"type": "object"}
        }),
        vec!["email"],
    )
}

fn list_lists() -> McpToolSchema {
    read_tool(
        "sendgrid.list_lists",
        "List SendGrid Marketing Campaigns contact lists.",
        serde_json::json!({
            "page_size": {"type": "integer", "minimum": 1, "maximum": 200},
            "page_token": {"type": "string"}
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
    fn only_send_email_requires_approval() {
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, tool.name == "sendgrid.send_email");
        }
    }
}
