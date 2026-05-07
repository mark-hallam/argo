//! MCP tool schemas for read-only Stripe events.

use argo_mcp::McpToolSchema;

/// All Stripe events tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_events(),
        get_event(),
        list_charges(),
        list_customers(),
        get_customer(),
        list_subscriptions(),
    ]
}

fn list_events() -> McpToolSchema {
    read_tool(
        "stripe_events.list_events",
        "List Stripe events visible to the restricted API key.",
        with_created(serde_json::json!({
            "type": {"type": "string"}
        })),
        Vec::new(),
    )
}

fn get_event() -> McpToolSchema {
    read_tool(
        "stripe_events.get_event",
        "Fetch one Stripe event by id.",
        serde_json::json!({"id": {"type": "string"}}),
        vec!["id"],
    )
}

fn list_charges() -> McpToolSchema {
    read_tool(
        "stripe_events.list_charges",
        "List Stripe charges for observability.",
        with_created(serde_json::json!({
            "customer": {"type": "string"},
            "payment_intent": {"type": "string"}
        })),
        Vec::new(),
    )
}

fn list_customers() -> McpToolSchema {
    read_tool(
        "stripe_events.list_customers",
        "List Stripe customers.",
        list_props(serde_json::json!({
            "email": {"type": "string"}
        })),
        Vec::new(),
    )
}

fn get_customer() -> McpToolSchema {
    read_tool(
        "stripe_events.get_customer",
        "Fetch one Stripe customer by id.",
        serde_json::json!({"id": {"type": "string"}}),
        vec!["id"],
    )
}

fn list_subscriptions() -> McpToolSchema {
    read_tool(
        "stripe_events.list_subscriptions",
        "List Stripe subscriptions.",
        list_props(serde_json::json!({
            "customer": {"type": "string"},
            "status": {"type": "string"},
            "price": {"type": "string"}
        })),
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

fn with_created(mut props: serde_json::Value) -> serde_json::Value {
    props["created"] = serde_json::json!({"type": "integer", "minimum": 0});
    props["created_gte"] = serde_json::json!({"type": "integer", "minimum": 0});
    props["created_lte"] = serde_json::json!({"type": "integer", "minimum": 0});
    list_props(props)
}

fn list_props(mut props: serde_json::Value) -> serde_json::Value {
    props["limit"] =
        serde_json::json!({"type": "integer", "minimum": 1, "maximum": 100, "default": 10});
    props["starting_after"] = serde_json::json!({"type": "string"});
    props["ending_before"] = serde_json::json!({"type": "string"});
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
    fn every_list_tool_exposes_cursor_params() {
        for tool in all() {
            if tool.name.contains(".list_") {
                let props = &tool.input_schema["properties"];
                assert!(props["limit"].is_object());
                assert!(props["starting_after"].is_object());
                assert!(props["ending_before"].is_object());
            }
        }
    }
}
