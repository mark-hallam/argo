//! MCP tool schemas for HubSpot CRM.
use argo_mcp::McpToolSchema;

/// All HubSpot tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        search_contacts(),
        create_contact(),
        create_deal(),
        log_activity(),
        get_pipeline(),
    ]
}

fn search_contacts() -> McpToolSchema {
    McpToolSchema {
        name: "hubspot.search_contacts".into(),
        description: "Search HubSpot contacts using the CRM v3 search endpoint.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10},
                "after": {"type": "string"},
                "properties": {"type": "array", "items": {"type": "string"}}
            }
        }),
    }
}

fn create_contact() -> McpToolSchema {
    McpToolSchema {
        name: "hubspot.create_contact".into(),
        description: "Create a HubSpot contact from a properties object.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "properties": {"type": "object", "additionalProperties": {"type": "string"}}
            },
            "required": ["properties"]
        }),
    }
}

fn create_deal() -> McpToolSchema {
    McpToolSchema {
        name: "hubspot.create_deal".into(),
        description: "Create a HubSpot deal; optional associations are passed through.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "properties": {"type": "object", "additionalProperties": {"type": "string"}},
                "associations": {"type": "array"}
            },
            "required": ["properties"]
        }),
    }
}

fn log_activity() -> McpToolSchema {
    McpToolSchema {
        name: "hubspot.log_activity".into(),
        description: "Log a HubSpot activity as a note object.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "note_body": {"type": "string"},
                "timestamp": {"type": "string", "description": "Unix milliseconds."},
                "properties": {"type": "object", "additionalProperties": {"type": "string"}},
                "associations": {"type": "array"}
            },
            "required": ["note_body"]
        }),
    }
}

fn get_pipeline() -> McpToolSchema {
    McpToolSchema {
        name: "hubspot.get_pipeline".into(),
        description: "List HubSpot pipelines for an object type or fetch one pipeline.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "object_type": {"type": "string", "default": "deals"},
                "pipeline_id": {"type": "string"}
            }
        }),
    }
}
