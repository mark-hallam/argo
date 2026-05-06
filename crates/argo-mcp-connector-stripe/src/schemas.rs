//! MCP tool schemas for Stripe.
use argo_mcp::McpToolSchema;

/// All Stripe tool schemas in stable order.
pub fn all() -> Vec<McpToolSchema> {
    vec![
        list_customers(),
        get_customer(),
        create_charge(),
        list_charges(),
        refund_charge(),
        create_invoice(),
        list_invoices(),
    ]
}

fn list_customers() -> McpToolSchema {
    McpToolSchema {
        name: "stripe.list_customers".into(),
        description: "List Stripe customers.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": false,
            "properties": {
                "email": {"type": "string"},
                "starting_after": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10}
            }
        }),
    }
}

fn get_customer() -> McpToolSchema {
    McpToolSchema {
        name: "stripe.get_customer".into(),
        description: "Fetch one Stripe customer by id.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": false,
            "properties": {"id": {"type": "string"}},
            "required": ["id"]
        }),
    }
}

fn create_charge() -> McpToolSchema {
    McpToolSchema {
        name: "stripe.create_charge".into(),
        description: "Create a Stripe charge. Requires operator approval.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": true,
            "properties": {
                "amount": {"type": "integer", "minimum": 1, "description": "Amount in the smallest currency unit."},
                "currency": {"type": "string"},
                "customer": {"type": "string"},
                "source": {"type": "string"},
                "description": {"type": "string"}
            },
            "required": ["amount", "currency"]
        }),
    }
}

fn list_charges() -> McpToolSchema {
    McpToolSchema {
        name: "stripe.list_charges".into(),
        description: "List Stripe charges.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": false,
            "properties": {
                "customer": {"type": "string"},
                "starting_after": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10}
            }
        }),
    }
}

fn refund_charge() -> McpToolSchema {
    McpToolSchema {
        name: "stripe.refund_charge".into(),
        description: "Refund a Stripe charge. Requires operator approval.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": true,
            "properties": {
                "charge": {"type": "string"},
                "amount": {"type": "integer", "minimum": 1},
                "reason": {"type": "string", "enum": ["duplicate", "fraudulent", "requested_by_customer"]}
            },
            "required": ["charge"]
        }),
    }
}

fn create_invoice() -> McpToolSchema {
    McpToolSchema {
        name: "stripe.create_invoice".into(),
        description: "Create a Stripe invoice. Requires operator approval.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": true,
            "properties": {
                "customer": {"type": "string"},
                "description": {"type": "string"},
                "collection_method": {"type": "string", "enum": ["charge_automatically", "send_invoice"]},
                "auto_advance": {"type": "boolean"}
            },
            "required": ["customer"]
        }),
    }
}

fn list_invoices() -> McpToolSchema {
    McpToolSchema {
        name: "stripe.list_invoices".into(),
        description: "List Stripe invoices.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "approval_required": false,
            "properties": {
                "customer": {"type": "string"},
                "status": {"type": "string"},
                "starting_after": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10}
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_tools_are_marked_approval_required() {
        let tools = all();
        let mutating = [
            "stripe.create_charge",
            "stripe.refund_charge",
            "stripe.create_invoice",
        ];
        for tool in tools {
            let required = tool.input_schema["approval_required"].as_bool().unwrap();
            assert_eq!(required, mutating.contains(&tool.name.as_str()));
        }
    }
}
