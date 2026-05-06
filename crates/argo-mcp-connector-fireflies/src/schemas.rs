//! MCP tool schemas for Fireflies.ai.
use argo_mcp::McpToolSchema;

/// All Fireflies.ai tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_meetings(),
        get_meeting(),
        get_transcript(),
        search_transcripts(),
        list_users(),
    ]
}

fn list_meetings() -> McpToolSchema {
    read_tool(
        "fireflies.list_meetings",
        "List Fireflies.ai meetings and transcript metadata.",
        transcript_filter_props(false),
        Vec::new(),
    )
}

fn get_meeting() -> McpToolSchema {
    read_tool(
        "fireflies.get_meeting",
        "Fetch one Fireflies.ai meeting summary by transcript id.",
        id_props(),
        vec!["meeting_id"],
    )
}

fn get_transcript() -> McpToolSchema {
    read_tool(
        "fireflies.get_transcript",
        "Fetch one Fireflies.ai transcript with sentence-level text.",
        id_props(),
        vec!["meeting_id"],
    )
}

fn search_transcripts() -> McpToolSchema {
    read_tool(
        "fireflies.search_transcripts",
        "Search Fireflies.ai transcripts by passing keyword filters to GraphQL.",
        transcript_filter_props(true),
        vec!["keyword"],
    )
}

fn list_users() -> McpToolSchema {
    read_tool(
        "fireflies.list_users",
        "List Fireflies.ai team users visible to the API key.",
        serde_json::json!({}),
        Vec::new(),
    )
}

fn read_tool(
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
            "approval_required": false,
            "properties": properties,
            "required": required
        }),
    }
}

fn id_props() -> serde_json::Value {
    serde_json::json!({
        "meeting_id": {"type": "string"},
        "transcript_id": {"type": "string"}
    })
}

fn transcript_filter_props(include_keyword: bool) -> serde_json::Value {
    let mut props = serde_json::json!({
        "from_date": {"type": "string", "description": "ISO-8601 lower bound."},
        "to_date": {"type": "string", "description": "ISO-8601 upper bound."},
        "date": {"type": "number", "description": "Deprecated Fireflies millisecond date filter."},
        "limit": {"type": "integer", "minimum": 1, "maximum": 50, "default": 25},
        "skip": {"type": "integer", "minimum": 0, "default": 0},
        "host_email": {"type": "string"},
        "user_id": {"type": "string"},
        "mine": {"type": "boolean"},
        "organizers": {"type": "array", "items": {"type": "string"}},
        "participants": {"type": "array", "items": {"type": "string"}},
        "channel_id": {"type": "string"}
    });
    if include_keyword {
        props["keyword"] = serde_json::json!({"type": "string"});
        props["scope"] =
            serde_json::json!({"type": "string", "enum": ["title", "sentences", "all"]});
    }
    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_are_autonomous_safe() {
        for tool in all() {
            assert_eq!(tool.input_schema["approval_required"], false);
        }
    }

    #[test]
    fn search_exposes_graphql_keyword_filters() {
        let schema = search_transcripts();
        assert_eq!(schema.input_schema["required"][0], "keyword");
        assert!(schema.input_schema["properties"]["scope"]["enum"].is_array());
    }
}
