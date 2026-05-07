//! MCP tool schemas for Discord.

use argo_mcp::McpToolSchema;

/// All Discord tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_guilds(),
        list_channels(),
        list_messages(),
        get_message(),
        send_message(),
        add_reaction(),
        list_members(),
    ]
}

fn list_guilds() -> McpToolSchema {
    read_tool(
        "discord.list_guilds",
        "List guilds visible to the bot.",
        page_props(false),
        Vec::new(),
    )
}

fn list_channels() -> McpToolSchema {
    read_tool(
        "discord.list_channels",
        "List channels for one Discord guild.",
        serde_json::json!({"guild_id": {"type": "string"}}),
        vec!["guild_id"],
    )
}

fn list_messages() -> McpToolSchema {
    read_tool(
        "discord.list_messages",
        "List messages in one Discord channel.",
        serde_json::json!({
            "channel_id": {"type": "string"},
            "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 50},
            "before": {"type": "string"},
            "after": {"type": "string"}
        }),
        vec!["channel_id"],
    )
}

fn get_message() -> McpToolSchema {
    read_tool(
        "discord.get_message",
        "Fetch one Discord message.",
        message_ref_props(false),
        vec!["channel_id", "message_id"],
    )
}

fn send_message() -> McpToolSchema {
    write_tool(
        "discord.send_message",
        "Send a Discord message. Requires operator approval.",
        serde_json::json!({
            "channel_id": {"type": "string"},
            "content": {"type": "string"},
            "allowed_mentions": {"type": "object"},
            "message_reference": {"type": "object"}
        }),
        vec!["channel_id", "content"],
    )
}

fn add_reaction() -> McpToolSchema {
    write_tool(
        "discord.add_reaction",
        "Add a Discord message reaction. Requires operator approval.",
        serde_json::json!({
            "channel_id": {"type": "string"},
            "message_id": {"type": "string"},
            "emoji": {"type": "string"}
        }),
        vec!["channel_id", "message_id", "emoji"],
    )
}

fn list_members() -> McpToolSchema {
    read_tool(
        "discord.list_members",
        "List members for one Discord guild.",
        page_props(true),
        vec!["guild_id"],
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

fn page_props(include_guild: bool) -> serde_json::Value {
    let mut props = serde_json::json!({
        "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
        "before": {"type": "string"},
        "after": {"type": "string"}
    });
    if include_guild {
        props["guild_id"] = serde_json::json!({"type": "string"});
    }
    props
}

fn message_ref_props(include_emoji: bool) -> serde_json::Value {
    let mut props = serde_json::json!({
        "channel_id": {"type": "string"},
        "message_id": {"type": "string"}
    });
    if include_emoji {
        props["emoji"] = serde_json::json!({"type": "string"});
    }
    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = ["discord.send_message", "discord.add_reaction"];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }
}
