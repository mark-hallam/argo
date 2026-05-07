//! MCP tool schemas for Trello.

use argo_mcp::McpToolSchema;

/// All Trello tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_boards(),
        list_cards(),
        get_card(),
        create_card(),
        update_card(),
        add_comment(),
        move_card(),
    ]
}

fn list_boards() -> McpToolSchema {
    read_tool(
        "trello.list_boards",
        "List Trello boards visible to a member.",
        serde_json::json!({
            "member": {"type": "string", "default": "me"},
            "filter": {"type": "string", "default": "open"},
            "fields": {"type": "string"}
        }),
        Vec::new(),
    )
}

fn list_cards() -> McpToolSchema {
    read_tool(
        "trello.list_cards",
        "List Trello cards on a board.",
        serde_json::json!({
            "board_id": {"type": "string"},
            "filter": {"type": "string", "default": "open"},
            "fields": {"type": "string"}
        }),
        vec!["board_id"],
    )
}

fn get_card() -> McpToolSchema {
    read_tool(
        "trello.get_card",
        "Fetch one Trello card.",
        serde_json::json!({"card_id": {"type": "string"}}),
        vec!["card_id"],
    )
}

fn create_card() -> McpToolSchema {
    write_tool(
        "trello.create_card",
        "Create a Trello card. Requires operator approval.",
        serde_json::json!({
            "list_id": {"type": "string"},
            "name": {"type": "string"},
            "desc": {"type": "string"},
            "pos": {"type": "string"},
            "due": {"type": "string"}
        }),
        vec!["list_id", "name"],
    )
}

fn update_card() -> McpToolSchema {
    write_tool(
        "trello.update_card",
        "Update a Trello card. Requires operator approval.",
        serde_json::json!({
            "card_id": {"type": "string"},
            "name": {"type": "string"},
            "desc": {"type": "string"},
            "closed": {"type": "boolean"},
            "due": {"type": "string"},
            "list_id": {"type": "string"}
        }),
        vec!["card_id"],
    )
}

fn add_comment() -> McpToolSchema {
    write_tool(
        "trello.add_comment",
        "Add a comment to a Trello card. Requires operator approval.",
        serde_json::json!({
            "card_id": {"type": "string"},
            "text": {"type": "string"}
        }),
        vec!["card_id", "text"],
    )
}

fn move_card() -> McpToolSchema {
    write_tool(
        "trello.move_card",
        "Move a Trello card to another list. Requires operator approval.",
        serde_json::json!({
            "card_id": {"type": "string"},
            "list_id": {"type": "string"}
        }),
        vec!["card_id", "list_id"],
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
            "trello.create_card",
            "trello.update_card",
            "trello.add_comment",
            "trello.move_card",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }
}
