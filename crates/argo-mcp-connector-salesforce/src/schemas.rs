//! MCP tool schemas for Salesforce.
use argo_mcp::McpToolSchema;

/// All Salesforce tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        query(),
        get_object(),
        create_object(),
        update_object(),
        list_accounts(),
        list_opportunities(),
        add_note(),
    ]
}

fn query() -> McpToolSchema {
    read_tool(
        "salesforce.query",
        "Run a SOQL query passthrough.",
        serde_json::json!({"soql": {"type": "string"}}),
        vec!["soql"],
    )
}

fn get_object() -> McpToolSchema {
    read_tool(
        "salesforce.get_object",
        "Fetch one Salesforce object by API name and id.",
        object_id_props(),
        vec!["object", "id"],
    )
}

fn create_object() -> McpToolSchema {
    write_tool(
        "salesforce.create_object",
        "Create a Salesforce object. Requires operator approval.",
        object_fields_props(false),
        vec!["object", "fields"],
    )
}

fn update_object() -> McpToolSchema {
    write_tool(
        "salesforce.update_object",
        "Update a Salesforce object. Requires operator approval.",
        object_fields_props(true),
        vec!["object", "id", "fields"],
    )
}

fn list_accounts() -> McpToolSchema {
    read_tool(
        "salesforce.list_accounts",
        "List Salesforce accounts.",
        limit_props(),
        Vec::new(),
    )
}

fn list_opportunities() -> McpToolSchema {
    read_tool(
        "salesforce.list_opportunities",
        "List Salesforce opportunities.",
        limit_props(),
        Vec::new(),
    )
}

fn add_note() -> McpToolSchema {
    write_tool(
        "salesforce.add_note",
        "Add a note to a parent Salesforce record. Requires operator approval.",
        serde_json::json!({
            "parent_id": {"type": "string"},
            "title": {"type": "string"},
            "body": {"type": "string"}
        }),
        vec!["parent_id", "body"],
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

fn object_id_props() -> serde_json::Value {
    serde_json::json!({
        "object": {"type": "string", "description": "Salesforce API object name, e.g. Account."},
        "id": {"type": "string"}
    })
}

fn object_fields_props(include_id: bool) -> serde_json::Value {
    let mut props = serde_json::json!({
        "object": {"type": "string"},
        "fields": {"type": "object"}
    });
    if include_id {
        props["id"] = serde_json::json!({"type": "string"});
    }
    props
}

fn limit_props() -> serde_json::Value {
    serde_json::json!({
        "limit": {"type": "integer", "minimum": 1, "maximum": 200, "default": 20}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = [
            "salesforce.create_object",
            "salesforce.update_object",
            "salesforce.add_note",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }
}
