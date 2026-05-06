//! MCP tool schemas for PagerDuty.
use argo_mcp::McpToolSchema;

/// All PagerDuty tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_incidents(),
        get_incident(),
        create_incident(),
        acknowledge_incident(),
        resolve_incident(),
        list_services(),
        list_oncalls(),
    ]
}

fn list_incidents() -> McpToolSchema {
    read_tool(
        "pagerduty.list_incidents",
        "List PagerDuty incidents.",
        incident_query_props(),
        Vec::new(),
    )
}

fn get_incident() -> McpToolSchema {
    read_tool(
        "pagerduty.get_incident",
        "Fetch one PagerDuty incident.",
        serde_json::json!({"incident_id": {"type": "string"}}),
        vec!["incident_id"],
    )
}

fn create_incident() -> McpToolSchema {
    write_tool(
        "pagerduty.create_incident",
        "Create a PagerDuty incident. Requires operator approval.",
        serde_json::json!({
            "title": {"type": "string"},
            "service_id": {"type": "string"},
            "details": {"type": "string"},
            "urgency": {"type": "string", "enum": ["high", "low"]},
            "priority_id": {"type": "string"},
            "incident_key": {"type": "string"},
            "from_email": {"type": "string"}
        }),
        vec!["title", "service_id"],
    )
}

fn acknowledge_incident() -> McpToolSchema {
    status_tool(
        "pagerduty.acknowledge_incident",
        "Acknowledge a PagerDuty incident. Requires operator approval.",
    )
}

fn resolve_incident() -> McpToolSchema {
    status_tool(
        "pagerduty.resolve_incident",
        "Resolve a PagerDuty incident. Requires operator approval.",
    )
}

fn list_services() -> McpToolSchema {
    read_tool(
        "pagerduty.list_services",
        "List PagerDuty services.",
        service_query_props(),
        Vec::new(),
    )
}

fn list_oncalls() -> McpToolSchema {
    read_tool(
        "pagerduty.list_oncalls",
        "List PagerDuty on-call entries.",
        oncall_query_props(),
        Vec::new(),
    )
}

fn status_tool(name: &str, description: &str) -> McpToolSchema {
    write_tool(
        name,
        description,
        serde_json::json!({
            "incident_id": {"type": "string"},
            "from_email": {"type": "string"}
        }),
        vec!["incident_id"],
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

fn incident_query_props() -> serde_json::Value {
    let mut props = paging_map();
    for (key, value) in serde_json::json!({
        "since": {"type": "string"},
        "until": {"type": "string"},
        "date_range": {"type": "string"},
        "incident_key": {"type": "string"},
        "statuses": {"type": "array", "items": {"type": "string"}},
        "service_ids": {"type": "array", "items": {"type": "string"}},
        "team_ids": {"type": "array", "items": {"type": "string"}},
        "user_ids": {"type": "array", "items": {"type": "string"}},
        "urgencies": {"type": "array", "items": {"type": "string"}},
        "sort_by": {"type": "string"},
        "time_zone": {"type": "string"},
        "include": {"type": "array", "items": {"type": "string"}}
    })
    .as_object()
    .into_iter()
    .flatten()
    {
        props.insert(key.clone(), value.clone());
    }
    serde_json::Value::Object(props)
}

fn service_query_props() -> serde_json::Value {
    let mut props = paging_map();
    props.insert("query".into(), serde_json::json!({"type": "string"}));
    props.insert("sort_by".into(), serde_json::json!({"type": "string"}));
    props.insert(
        "team_ids".into(),
        serde_json::json!({"type": "array", "items": {"type": "string"}}),
    );
    props.insert(
        "include".into(),
        serde_json::json!({"type": "array", "items": {"type": "string"}}),
    );
    serde_json::Value::Object(props)
}

fn oncall_query_props() -> serde_json::Value {
    let mut props = paging_map();
    for field in ["since", "until", "time_zone"] {
        props.insert(field.into(), serde_json::json!({"type": "string"}));
    }
    props.insert("earliest".into(), serde_json::json!({"type": "boolean"}));
    for field in [
        "user_ids",
        "escalation_policy_ids",
        "schedule_ids",
        "include",
    ] {
        props.insert(
            field.into(),
            serde_json::json!({"type": "array", "items": {"type": "string"}}),
        );
    }
    serde_json::Value::Object(props)
}

fn paging_map() -> serde_json::Map<String, serde_json::Value> {
    serde_json::json!({
        "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 25},
        "offset": {"type": "integer", "minimum": 0},
        "total": {"type": "boolean"}
    })
    .as_object()
    .cloned()
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let write_tools = [
            "pagerduty.create_incident",
            "pagerduty.acknowledge_incident",
            "pagerduty.resolve_incident",
        ];
        for tool in all() {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, write_tools.contains(&tool.name.as_str()));
        }
    }
}
