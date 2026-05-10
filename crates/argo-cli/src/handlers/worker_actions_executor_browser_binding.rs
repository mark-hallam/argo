//! `executor.binding` parser + template helpers for the action-level
//! browser executor. Split out of
//! [`super::worker_actions_executor_browser`] to keep that file under
//! the per-file 500-LOC budget.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor_mcp::substitute_args;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::ActionDeclaration;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserMode {
    /// Computer-use loop: hand the rendered `instruction_template`
    /// to the browser MCP's `cu_loop` tool and let it drive.
    Cu,
    /// Scripted CDP sequence: walk `binding.steps` and dispatch each
    /// to the matching browser MCP tool.
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionScope {
    /// Default. Sticky per (tenant, worker) — login + cookies persist
    /// across sequential actions.
    PerWorker,
    /// Ephemeral. Pool entry is dropped after dispatch.
    PerAction,
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserStep {
    pub action: String,
    pub selector: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserBinding {
    pub target_url: String,
    pub mode: BrowserMode,
    pub instruction_template: Option<String>,
    pub steps: Vec<BrowserStep>,
    pub session_scope: SessionScope,
}

/// Parse `executor.binding` for browser actions. Required:
/// `target_url`. Optional: `mode` (default `"cu"`),
/// `instruction_template`, `steps`, `session_scope` (default
/// `"per_worker"`). Templates are rendered just before each tool
/// call so secret + payload values land in the outbound request, not
/// the parsed binding.
///
/// # Example
/// ```ignore
/// // Inside the argo-cli crate:
/// // let binding = parse_binding(&decl, &action)?;
/// // assert_eq!(binding.target_url, "https://example.com/post");
/// ```
pub(crate) fn parse_binding(decl: &ActionDeclaration) -> ArgoResult<BrowserBinding> {
    let raw = decl
        .executor
        .api_binding()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ArgoError::Provider(
                "browser executor missing `executor.binding`; manifest must \
                 declare `executor = { kind = \"browser\", binding = \
                 '{\"target_url\": \"...\"}' }`"
                    .into(),
            )
        })?;
    let value: Value = if raw.starts_with('{') {
        serde_json::from_str(raw).map_err(|e| {
            ArgoError::Provider(format!("browser executor binding JSON parse failed: {e}"))
        })?
    } else if raw.starts_with("http://") || raw.starts_with("https://") {
        json!({"target_url": raw})
    } else {
        return Err(ArgoError::Provider(format!(
            "browser executor binding '{raw}' is not yet implemented; \
             use a JSON object with `target_url` or a bare URL"
        )));
    };
    let target_url = value
        .get("target_url")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            ArgoError::Provider(
                "browser executor binding missing required `target_url`".into(),
            )
        })?
        .to_string();
    let mode = match value
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("cu")
        .to_ascii_lowercase()
        .as_str()
    {
        "cu" => BrowserMode::Cu,
        "script" => BrowserMode::Script,
        other => {
            return Err(ArgoError::Provider(format!(
                "browser executor binding has unknown mode '{other}' — valid: cu | script"
            )));
        }
    };
    let session_scope = match value
        .get("session_scope")
        .and_then(Value::as_str)
        .unwrap_or("per_worker")
        .to_ascii_lowercase()
        .as_str()
    {
        "per_worker" => SessionScope::PerWorker,
        "per_action" => SessionScope::PerAction,
        other => {
            return Err(ArgoError::Provider(format!(
                "browser executor binding has unknown session_scope '{other}' — \
                 valid: per_worker | per_action"
            )));
        }
    };
    let instruction_template = value
        .get("instruction_template")
        .and_then(Value::as_str)
        .map(str::to_string);
    let steps = parse_steps(value.get("steps"))?;
    Ok(BrowserBinding {
        target_url,
        mode,
        instruction_template,
        steps,
        session_scope,
    })
}

fn parse_steps(value: Option<&Value>) -> ArgoResult<Vec<BrowserStep>> {
    let Some(Value::Array(items)) = value else {
        return Ok(Vec::new());
    };
    let mut steps = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        let Value::Object(map) = item else {
            return Err(ArgoError::Provider(format!(
                "browser executor binding step[{idx}] must be a JSON object"
            )));
        };
        let action = map
            .get("action")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                ArgoError::Provider(format!(
                    "browser executor binding step[{idx}] missing `action`"
                ))
            })?
            .to_string();
        let selector = map
            .get("selector")
            .and_then(Value::as_str)
            .map(str::to_string);
        let value = map
            .get("value")
            .and_then(Value::as_str)
            .map(str::to_string);
        steps.push(BrowserStep {
            action,
            selector,
            value,
        });
    }
    Ok(steps)
}

/// Render a string template through the same payload-aware
/// substitution engine used by the MCP / API executors, plus a
/// `{{ secret.NAME }}` pass that pulls credentials from
/// `argo-config::cred` just before the outbound tool call.
pub(crate) fn render_string(template: &str, action: &ActionRecord) -> String {
    let (protected, secrets) = protect_secrets(template);
    let rendered = substitute_args(&Value::String(protected), action);
    let body = rendered.as_str().unwrap_or(template).to_string();
    fill_secrets(body, &secrets)
}

fn protect_secrets(body: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(body.len());
    let mut secrets = Vec::new();
    let mut cursor = 0;
    while let Some(start) = body[cursor..].find("{{") {
        out.push_str(&body[cursor..cursor + start]);
        let open = cursor + start;
        let Some(rel_end) = body[open + 2..].find("}}") else {
            out.push_str(&body[open..]);
            return (out, secrets);
        };
        let close = open + 2 + rel_end;
        let key = body[open + 2..close].trim();
        if let Some(secret) = key.strip_prefix("secret.") {
            let idx = secrets.len();
            secrets.push(secret.to_string());
            out.push_str(&format!("@@ARGO_BROWSER_SECRET_{idx}@@"));
        } else {
            out.push_str(&body[open..close + 2]);
        }
        cursor = close + 2;
    }
    out.push_str(&body[cursor..]);
    (out, secrets)
}

fn fill_secrets(mut body: String, secrets: &[String]) -> String {
    for (idx, name) in secrets.iter().enumerate() {
        let token = format!("@@ARGO_BROWSER_SECRET_{idx}@@");
        let value = argo_config::cred::get(name).unwrap_or_else(|_| "(missing)".to_string());
        body = body.replace(&token, &value);
    }
    body
}
