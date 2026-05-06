//! MCP tool schemas for Airtable.
use argo_mcp::McpToolSchema;

/// All Airtable tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_bases(),
        list_tables(),
        list_records(),
        get_record(),
        create_record(),
        update_record(),
        delete_record(),
    ]
}

fn list_bases() -> McpToolSchema {
    read_tool(
        "airtable.list_bases",
        "List Airtable bases visible to the authenticated PAT.",
        paging_props(),
        Vec::new(),
    )
}

fn list_tables() -> McpToolSchema {
    read_tool(
        "airtable.list_tables",
        "List tables in one Airtable base.",
        serde_json::json!({"base_id": {"type": "string"}}),
        vec!["base_id"],
    )
}

fn list_records() -> McpToolSchema {
    read_tool(
        "airtable.list_records",
        "List records in one Airtable table. Passes filterByFormula through unchanged.",
        record_query_props(),
        vec!["base_id", "table"],
    )
}

fn get_record() -> McpToolSchema {
    read_tool(
        "airtable.get_record",
        "Fetch one Airtable record.",
        record_id_props(false),
        vec!["base_id", "table", "record_id"],
    )
}

fn create_record() -> McpToolSchema {
    write_tool(
        "airtable.create_record",
        "Create an Airtable record. Requires operator approval.",
        record_fields_props(false),
        vec!["base_id", "table", "fields"],
    )
}

fn update_record() -> McpToolSchema {
    write_tool(
        "airtable.update_record",
        "Update an Airtable record. Requires operator approval.",
        record_fields_props(true),
        vec!["base_id", "table", "record_id", "fields"],
    )
}

fn delete_record() -> McpToolSchema {
    write_tool(
        "airtable.delete_record",
        "Delete an Airtable record. Requires operator approval.",
        record_id_props(false),
        vec!["base_id", "table", "record_id"],
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

fn record_query_props() -> serde_json::Value {
    let mut props = table_props();
    let extra = serde_json::json!({
        "filterByFormula": {"type": "string"},
        "view": {"type": "string"},
        "fields": {"type": "array", "items": {"type": "string"}}
    });
    merge_props(&mut props, paging_props());
    merge_props(&mut props, extra);
    serde_json::Value::Object(props)
}

fn record_id_props(include_fields: bool) -> serde_json::Value {
    let mut props = table_props();
    props.insert("record_id".into(), serde_json::json!({"type": "string"}));
    if include_fields {
        props.insert("fields".into(), serde_json::json!({"type": "object"}));
    }
    serde_json::Value::Object(props)
}

fn record_fields_props(include_record_id: bool) -> serde_json::Value {
    let mut props = table_props();
    props.insert("fields".into(), serde_json::json!({"type": "object"}));
    props.insert("typecast".into(), serde_json::json!({"type": "boolean"}));
    if include_record_id {
        props.insert("record_id".into(), serde_json::json!({"type": "string"}));
    }
    serde_json::Value::Object(props)
}

fn table_props() -> serde_json::Map<String, serde_json::Value> {
    serde_json::json!({
        "base_id": {"type": "string"},
        "table": {"type": "string", "description": "Table id or table name."},
        "table_id": {"type": "string", "description": "Alias accepted for table."}
    })
    .as_object()
    .cloned()
    .unwrap_or_default()
}

fn paging_props() -> serde_json::Value {
    serde_json::json!({
        "page_size": {"type": "integer", "minimum": 1, "maximum": 100},
        "max_records": {"type": "integer", "minimum": 1, "maximum": 1000},
        "offset": {"type": "string"}
    })
}

fn merge_props(props: &mut serde_json::Map<String, serde_json::Value>, extra: serde_json::Value) {
    if let Some(extra) = extra.as_object() {
        for (key, value) in extra {
            props.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = [
            "airtable.create_record",
            "airtable.update_record",
            "airtable.delete_record",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }

    #[test]
    fn list_records_exposes_filter_formula() {
        let schema = list_records();
        assert!(schema.input_schema["properties"]["filterByFormula"].is_object());
    }
}
