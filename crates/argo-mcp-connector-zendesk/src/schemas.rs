//! MCP tool schemas for Zendesk Support.

use argo_mcp::McpToolSchema;

/// All Zendesk tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_tickets(),
        get_ticket(),
        create_ticket(),
        update_ticket(),
        add_comment(),
        list_users(),
        list_macros(),
    ]
}

fn list_tickets() -> McpToolSchema {
    read_tool(
        "zendesk.list_tickets",
        "Search Zendesk tickets with a raw Zendesk search query.",
        serde_json::json!({"query": {"type": "string"}}),
        vec!["query"],
    )
}

fn get_ticket() -> McpToolSchema {
    read_tool(
        "zendesk.get_ticket",
        "Fetch one Zendesk ticket by id.",
        serde_json::json!({"id": {"type": "string"}}),
        vec!["id"],
    )
}

fn create_ticket() -> McpToolSchema {
    write_tool(
        "zendesk.create_ticket",
        "Create a Zendesk ticket. Requires operator approval.",
        serde_json::json!({"ticket": {"type": "object"}}),
        vec!["ticket"],
    )
}

fn update_ticket() -> McpToolSchema {
    write_tool(
        "zendesk.update_ticket",
        "Update a Zendesk ticket. Requires operator approval.",
        serde_json::json!({
            "id": {"type": "string"},
            "ticket": {"type": "object"}
        }),
        vec!["id", "ticket"],
    )
}

fn add_comment() -> McpToolSchema {
    write_tool(
        "zendesk.add_comment",
        "Add a Zendesk ticket comment. Requires operator approval.",
        serde_json::json!({
            "ticket_id": {"type": "string"},
            "body": {"type": "string"},
            "public": {"type": "boolean", "default": false}
        }),
        vec!["ticket_id", "body"],
    )
}

fn list_users() -> McpToolSchema {
    read_tool(
        "zendesk.list_users",
        "List Zendesk users.",
        serde_json::json!({
            "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 25},
            "role": {"type": "string"}
        }),
        Vec::new(),
    )
}

fn list_macros() -> McpToolSchema {
    read_tool(
        "zendesk.list_macros",
        "List Zendesk macros.",
        serde_json::json!({
            "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 25},
            "active": {"type": "boolean"}
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
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = [
            "zendesk.create_ticket",
            "zendesk.update_ticket",
            "zendesk.add_comment",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }
}
