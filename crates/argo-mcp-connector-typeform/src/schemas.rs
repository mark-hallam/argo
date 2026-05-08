//! MCP tool schemas for Typeform.

use argo_mcp::McpToolSchema;

/// All Typeform tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_forms(),
        get_form(),
        list_responses(),
        get_response(),
        create_form(),
        update_form(),
    ]
}

fn list_forms() -> McpToolSchema {
    read_tool(
        "typeform.list_forms",
        "List Typeform forms.",
        serde_json::json!({
            "page_size": {"type": "integer", "minimum": 1, "maximum": 1000, "default": 25},
            "search": {"type": "string"},
            "workspace_id": {"type": "string"}
        }),
        Vec::new(),
    )
}

fn get_form() -> McpToolSchema {
    read_tool(
        "typeform.get_form",
        "Fetch one Typeform form by id.",
        serde_json::json!({"form_id": {"type": "string"}}),
        vec!["form_id"],
    )
}

fn list_responses() -> McpToolSchema {
    read_tool(
        "typeform.list_responses",
        "List Typeform responses for one form.",
        serde_json::json!({
            "form_id": {"type": "string"},
            "page_size": {"type": "integer", "minimum": 1, "maximum": 1000, "default": 25},
            "since": {"type": "string"},
            "until": {"type": "string"}
        }),
        vec!["form_id"],
    )
}

fn get_response() -> McpToolSchema {
    read_tool(
        "typeform.get_response",
        "Fetch one Typeform response by id.",
        serde_json::json!({
            "form_id": {"type": "string"},
            "response_id": {"type": "string"}
        }),
        vec!["form_id", "response_id"],
    )
}

fn create_form() -> McpToolSchema {
    write_tool(
        "typeform.create_form",
        "Create a Typeform form. Requires operator approval.",
        serde_json::json!({"form": {"type": "object"}}),
        vec!["form"],
    )
}

fn update_form() -> McpToolSchema {
    write_tool(
        "typeform.update_form",
        "Update a Typeform form. Requires operator approval.",
        serde_json::json!({
            "form_id": {"type": "string"},
            "form": {"type": "object"}
        }),
        vec!["form_id", "form"],
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
        let write_tools = ["typeform.create_form", "typeform.update_form"];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }
}
