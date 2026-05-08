//! MCP tool schemas for Figma.

use argo_mcp::McpToolSchema;

/// All Figma tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_files(),
        get_file(),
        get_file_components(),
        list_comments(),
        add_comment(),
    ]
}

fn list_files() -> McpToolSchema {
    read_tool(
        "figma.list_files",
        "List files in a Figma project.",
        serde_json::json!({"project_id": {"type": "string"}}),
        vec!["project_id"],
    )
}

fn get_file() -> McpToolSchema {
    read_tool(
        "figma.get_file",
        "Fetch one Figma file by file key.",
        file_props_with_options(),
        vec!["file_key"],
    )
}

fn get_file_components() -> McpToolSchema {
    read_tool(
        "figma.get_file_components",
        "List components declared in a Figma file.",
        serde_json::json!({"file_key": {"type": "string"}}),
        vec!["file_key"],
    )
}

fn list_comments() -> McpToolSchema {
    read_tool(
        "figma.list_comments",
        "List comments for a Figma file.",
        serde_json::json!({"file_key": {"type": "string"}}),
        vec!["file_key"],
    )
}

fn add_comment() -> McpToolSchema {
    write_tool(
        "figma.add_comment",
        "Add a Figma file comment. Requires operator approval.",
        serde_json::json!({
            "file_key": {"type": "string"},
            "message": {"type": "string"},
            "client_meta": {"type": "object"}
        }),
        vec!["file_key", "message"],
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

fn file_props_with_options() -> serde_json::Value {
    serde_json::json!({
        "file_key": {"type": "string"},
        "version": {"type": "string"},
        "ids": {"type": "string"},
        "depth": {"type": "string"},
        "geometry": {"type": "string"}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_add_comment_requires_approval() {
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, tool.name == "figma.add_comment");
        }
    }
}
