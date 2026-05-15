//! Stripe MCP connector for customers, payments, invoices, subscriptions, and refunds.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_stripe::StripeService;
//! let svc = StripeService;
//! assert!(svc.list_tools().len() >= 10);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "stripe.list_customers".into(),
//!     arguments: serde_json::json!({"limit": 3}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
pub mod auth;

/// Backwards-compatible name for the Stripe REST client.
pub type StripeClient = StripeRestClient;
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Stripe MCP service.
pub struct StripeService;

/// Thin Stripe REST client authenticated with a bearer secret key.
#[derive(Clone)]
pub struct StripeRestClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl StripeRestClient {
    /// Build from Argo credentials (`STRIPE_API_KEY`) and optional `STRIPE_BASE_URL`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::api_key_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit API key and base URL.
    pub fn new(api_key: String, base_url: String) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("missing {}", auth::STRIPE_API_KEY_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-stripe/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("stripe http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    async fn call(
        &self,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match tool {
            "stripe.list_customers" => self.get("/v1/customers", list_query(args)).await,
            "stripe.get_customer" => self.get_id("/v1/customers", args, "id").await,
            "stripe.create_charge" => {
                let mut form = required_pairs(
                    args,
                    &[("amount", ValueKind::U64), ("currency", ValueKind::Str)],
                )?;
                push_str(args, &mut form, "customer");
                push_str(args, &mut form, "source");
                push_str(args, &mut form, "description");
                self.post_form("/v1/charges", form).await
            }
            "stripe.list_charges" => self.get("/v1/charges", list_query(args)).await,
            "stripe.create_payment_intent" => {
                let mut form = required_pairs(
                    args,
                    &[("amount", ValueKind::U64), ("currency", ValueKind::Str)],
                )?;
                push_str(args, &mut form, "customer");
                push_str(args, &mut form, "description");
                self.post_form("/v1/payment_intents", form).await
            }
            "stripe.list_payment_intents" => {
                self.get("/v1/payment_intents", list_query(args)).await
            }
            "stripe.create_refund" => {
                let mut form = required_pairs(args, &[("payment_intent", ValueKind::Str)])?;
                push_u64(args, &mut form, "amount");
                push_str(args, &mut form, "reason");
                self.post_form("/v1/refunds", form).await
            }
            "stripe.refund_charge" => {
                let mut form = required_pairs(args, &[("charge", ValueKind::Str)])?;
                push_u64(args, &mut form, "amount");
                push_str(args, &mut form, "reason");
                self.post_form("/v1/refunds", form).await
            }
            "stripe.list_refunds" => self.get("/v1/refunds", list_query(args)).await,
            "stripe.create_invoice" => {
                let mut form = required_pairs(args, &[("customer", ValueKind::Str)])?;
                push_bool(args, &mut form, "auto_advance");
                push_str(args, &mut form, "collection_method");
                push_str(args, &mut form, "description");
                self.post_form("/v1/invoices", form).await
            }
            "stripe.list_invoices" => self.get("/v1/invoices", list_query(args)).await,
            "stripe.create_subscription" => {
                let mut form = required_pairs(
                    args,
                    &[("customer", ValueKind::Str), ("price", ValueKind::Str)],
                )?;
                push_bool(args, &mut form, "trial_from_plan");
                self.post_form("/v1/subscriptions", form).await
            }
            "stripe.list_subscriptions" => self.get("/v1/subscriptions", list_query(args)).await,
            "stripe.cancel_subscription" => {
                let id = clean_id(&required_str(args, "id")?)?;
                self.delete(&format!("/v1/subscriptions/{id}")).await
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }

    async fn get_id(
        &self,
        prefix: &str,
        args: &serde_json::Value,
        field: &str,
    ) -> Result<serde_json::Value, String> {
        let id = clean_id(&required_str(args, field)?)?;
        self.get(&format!("{prefix}/{id}"), Vec::new()).await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("stripe GET {path}: {e}"))?;
        decode_response("GET", path, resp).await
    }

    async fn post_form(
        &self,
        path: &str,
        form: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("stripe POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }

    async fn delete(&self, path: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .delete(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| format!("stripe DELETE {path}: {e}"))?;
        decode_response("DELETE", path, resp).await
    }
}

#[async_trait]
impl McpServerService for StripeService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-stripe".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        TOOLS
            .iter()
            .map(|tool| McpToolSchema {
                name: tool.name.into(),
                description: tool.description.into(),
                input_schema: schema(tool),
            })
            .collect()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match StripeRestClient::from_env() {
            Ok(client) => client,
            Err(text) => return Ok(text_result(text, true)),
        };
        Ok(to_call_result(
            client.call(&params.name, &params.arguments).await,
        ))
    }
}

fn to_call_result(result: Result<serde_json::Value, String>) -> CallToolResult {
    match result {
        Ok(value) => text_result(value.to_string(), false),
        Err(text) => text_result(text, true),
    }
}

fn text_result(text: String, is_error: bool) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error,
    }
}

struct ToolDef {
    name: &'static str,
    description: &'static str,
    approval_required: bool,
}

macro_rules! tool {
    ($name:literal, $description:literal, $approval:literal) => {
        ToolDef {
            name: $name,
            description: $description,
            approval_required: $approval,
        }
    };
}

const TOOLS: &[ToolDef] = &[
    tool!("stripe.list_customers", "List Stripe customers.", false),
    tool!(
        "stripe.get_customer",
        "Fetch one Stripe customer by id.",
        false
    ),
    tool!(
        "stripe.create_charge",
        "Create a Stripe charge. Requires operator approval.",
        true
    ),
    tool!("stripe.list_charges", "List Stripe charges.", false),
    tool!(
        "stripe.create_payment_intent",
        "Create a Stripe PaymentIntent. Requires operator approval.",
        true
    ),
    tool!(
        "stripe.list_payment_intents",
        "List Stripe PaymentIntents.",
        false
    ),
    tool!(
        "stripe.create_refund",
        "Create a Stripe refund. Requires operator approval.",
        true
    ),
    tool!(
        "stripe.refund_charge",
        "Refund a Stripe charge. Requires operator approval.",
        true
    ),
    tool!("stripe.list_refunds", "List Stripe refunds.", false),
    tool!(
        "stripe.create_invoice",
        "Create a Stripe invoice. Requires operator approval.",
        true
    ),
    tool!("stripe.list_invoices", "List Stripe invoices.", false),
    tool!(
        "stripe.create_subscription",
        "Create a Stripe subscription. Requires operator approval.",
        true
    ),
    tool!(
        "stripe.list_subscriptions",
        "List Stripe subscriptions.",
        false
    ),
    tool!(
        "stripe.cancel_subscription",
        "Cancel a Stripe subscription. Requires operator approval.",
        true
    ),
];

fn schema(tool: &ToolDef) -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "approval_required": tool.approval_required,
        "properties": {
            "id": {"type": "string"},
            "customer": {"type": "string"},
            "charge": {"type": "string"},
            "price": {"type": "string"},
            "amount": {"type": "integer", "minimum": 1},
            "currency": {"type": "string"},
            "source": {"type": "string"},
            "payment_intent": {"type": "string"},
            "reason": {"type": "string"},
            "description": {"type": "string"},
            "collection_method": {"type": "string"},
            "auto_advance": {"type": "boolean"},
            "trial_from_plan": {"type": "boolean"},
            "starting_after": {"type": "string"},
            "limit": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10}
        }
    })
}

async fn decode_response(
    method: &str,
    path: &str,
    resp: reqwest::Response,
) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("stripe {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("stripe {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("stripe {method} {path}: HTTP {status}: {parsed}"))
    }
}

enum ValueKind {
    Str,
    U64,
}

fn required_pairs(
    args: &serde_json::Value,
    fields: &[(&str, ValueKind)],
) -> Result<Vec<(String, String)>, String> {
    fields
        .iter()
        .map(|(field, kind)| match kind {
            ValueKind::Str => required_str(args, field).map(|value| ((*field).into(), value)),
            ValueKind::U64 => {
                required_u64(args, field).map(|value| ((*field).into(), value.to_string()))
            }
        })
        .collect()
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_u64(args: &serde_json::Value, field: &str) -> Result<u64, String> {
    args.get(field)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("missing integer field '{field}'"))
}

fn list_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let mut query = vec![(
        "limit".into(),
        args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .clamp(1, 100)
            .to_string(),
    )];
    push_str(args, &mut query, "customer");
    push_str(args, &mut query, "starting_after");
    query
}

fn push_str(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
        out.push((field.into(), value.to_string()));
    }
}

fn push_u64(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_u64()) {
        out.push((field.into(), value.to_string()));
    }
}

fn push_bool(args: &serde_json::Value, out: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = args.get(field).and_then(|v| v.as_bool()) {
        out.push((field.into(), value.to_string()));
    }
}

fn clean_id(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Stripe id: {raw}"))
    }
}

#[cfg(test)]
mod lib_tests;
