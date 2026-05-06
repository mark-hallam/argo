//! MCP tool schemas for Blue Bubbles iMessage.

use argo_mcp::McpToolSchema;

/// All iMessage tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        send_message(),
        list_chats(),
        list_messages(),
        get_message(),
        send_reaction(),
    ]
}

fn send_message() -> McpToolSchema {
    write_tool(
        "imessage.send_message",
        "Send an iMessage via Blue Bubbles. Requires operator approval.",
        serde_json::json!({
            "chat_guid": {"type": "string"},
            "message": {"type": "string"},
            "temp_guid": {"type": "string"}
        }),
        vec!["chat_guid", "message"],
    )
}

fn list_chats() -> McpToolSchema {
    read_tool(
        "imessage.list_chats",
        "List Blue Bubbles chats.",
        query_props(),
    )
}

fn list_messages() -> McpToolSchema {
    read_tool(
        "imessage.list_messages",
        "List Blue Bubbles messages with optional query filters.",
        serde_json::json!({
            "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
            "offset": {"type": "integer", "minimum": 0},
            "after": {"type": "integer"},
            "before": {"type": "integer"},
            "sort": {"type": "string"},
            "where": {"type": "array"}
        }),
    )
}

fn get_message() -> McpToolSchema {
    read_tool(
        "imessage.get_message",
        "Fetch one Blue Bubbles message by GUID.",
        serde_json::json!({"message_guid": {"type": "string"}}),
    )
}

fn send_reaction() -> McpToolSchema {
    write_tool(
        "imessage.send_reaction",
        "Send a Blue Bubbles tapback reaction. Requires operator approval.",
        serde_json::json!({
            "chat_guid": {"type": "string"},
            "selected_message_guid": {"type": "string"},
            "reaction": {"type": "string"},
            "part_index": {"type": "integer", "minimum": 0}
        }),
        vec!["chat_guid", "selected_message_guid", "reaction"],
    )
}

fn read_tool(name: &str, description: &str, properties: serde_json::Value) -> McpToolSchema {
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": false,
            "properties": properties
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

fn query_props() -> serde_json::Value {
    serde_json::json!({
        "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
        "offset": {"type": "integer", "minimum": 0},
        "with": {"type": "array", "items": {"type": "string"}},
        "sort": {"type": "string"}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_are_approval_gated_reads_are_safe() {
        for tool in all() {
            let approval = tool.input_schema["approval_required"].as_bool().unwrap();
            let should_require = matches!(
                tool.name.as_str(),
                "imessage.send_message" | "imessage.send_reaction"
            );
            assert_eq!(approval, should_require, "{}", tool.name);
        }
    }
}
