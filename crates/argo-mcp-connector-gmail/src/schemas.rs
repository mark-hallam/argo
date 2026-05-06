//! MCP tool schemas for Gmail.
use argo_mcp::McpToolSchema;

/// All Gmail tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![search(), read(), draft(), send(), label()]
}

fn search() -> McpToolSchema {
    McpToolSchema {
        name: "gmail.search".into(),
        description: "Search Gmail messages using Gmail query syntax.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "label_ids": {"type": "array", "items": {"type": "string"}},
                "page_token": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10}
            }
        }),
    }
}

fn read() -> McpToolSchema {
    McpToolSchema {
        name: "gmail.read".into(),
        description: "Read one Gmail message by id.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "format": {"type": "string", "enum": ["minimal", "full", "metadata", "raw"], "default": "full"}
            },
            "required": ["id"]
        }),
    }
}

fn draft() -> McpToolSchema {
    let schema = compose_schema();
    McpToolSchema {
        name: "gmail.draft".into(),
        description: "Create a Gmail draft from raw MIME or structured text fields.".into(),
        input_schema: schema,
    }
}

fn send() -> McpToolSchema {
    let schema = compose_schema();
    McpToolSchema {
        name: "gmail.send".into(),
        description: "Send a Gmail message from raw MIME or structured text fields.".into(),
        input_schema: schema,
    }
}

fn label() -> McpToolSchema {
    McpToolSchema {
        name: "gmail.label".into(),
        description: "Add and/or remove labels from a Gmail message.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "add_label_ids": {"type": "array", "items": {"type": "string"}},
                "remove_label_ids": {"type": "array", "items": {"type": "string"}}
            },
            "required": ["id"]
        }),
    }
}

fn compose_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "raw": {"type": "string", "description": "Raw RFC 2822 message; encoded by the connector."},
            "to": {"type": "string"},
            "cc": {"type": "string"},
            "bcc": {"type": "string"},
            "from": {"type": "string"},
            "subject": {"type": "string"},
            "body_text": {"type": "string"},
            "thread_id": {"type": "string"}
        },
        "anyOf": [
            {"required": ["raw"]},
            {"required": ["to", "subject", "body_text"]}
        ]
    })
}
