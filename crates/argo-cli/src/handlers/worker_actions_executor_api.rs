//! Action-level REST API executor (mission-wire-api-action-executor).

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::ActionOutcome;
use super::worker_actions_executor_mcp::substitute_args;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::ActionDeclaration;
use reqwest::Method;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

const DEFAULT_API_ACTION_TIMEOUT: Duration = Duration::from_secs(30);
const API_TIMEOUT_ENV: &str = "ARGO_API_ACTION_TIMEOUT_MS";

#[derive(Debug, Clone)]
struct ApiBinding {
    url: String,
    method: Method,
    headers: HashMap<String, String>,
    body_template: Option<String>,
}

/// Per-base-URL HTTP client pool.
pub(crate) type ApiPool = Mutex<HashMap<String, reqwest::Client>>;

/// Process-wide default pool.
pub(crate) fn default_pool() -> &'static ApiPool {
    static POOL: OnceLock<ApiPool> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Default wallclock timeout for API action dispatch.
pub(crate) fn default_timeout() -> Duration {
    std::env::var(API_TIMEOUT_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|ms| *ms > 0)
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_API_ACTION_TIMEOUT)
}

/// Dispatch a single API-executor action through the HTTP client pool.
pub(crate) async fn dispatch_action(
    action: &ActionRecord,
    cwd: &Path,
    decl: &ActionDeclaration,
    pool: &ApiPool,
    timeout: Duration,
) -> ArgoResult<ActionOutcome> {
    let binding = parse_binding(decl)?;
    let url = render_string(&binding.url, action)?;
    let method = binding.method.clone();
    let client = acquire_client(pool, &url).await?;
    let request = build_request(&client, method.clone(), &url, &binding, action)?;
    let response = match tokio::time::timeout(timeout, request.send()).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => return Err(provider(format!("API action HTTP dispatch failed: {e}"))),
        Err(_) => return Err(timeout_error(action, timeout)),
    };
    let status = response.status();
    let text = match tokio::time::timeout(timeout, response.text()).await {
        Ok(Ok(body)) => body,
        Ok(Err(e)) => return Err(provider(format!("API action read response failed: {e}"))),
        Err(_) => return Err(timeout_error(action, timeout)),
    };
    if !status.is_success() {
        return Err(provider(format!(
            "API action returned HTTP {status}: {}",
            body_excerpt(&text)
        )));
    }
    let body = parse_body(&text);
    Ok(ActionOutcome::Completed {
        bundle_path: cwd
            .join(".skycot")
            .join("outputs")
            .join(&action.job_id)
            .join("actions")
            .join(&action.action_id),
        result_json: json!({
            "outcome": "completed",
            "executor": "api",
            "method": method.as_str(),
            "url": url,
            "status": status.as_u16(),
            "body": body,
        }),
    })
}

/// Sync wrapper used by `ApiExecutor::dispatch`.
pub(crate) fn dispatch_blocking(
    action: &ActionRecord,
    cwd: &Path,
    decl: &ActionDeclaration,
) -> ArgoResult<ActionOutcome> {
    let _ = parse_binding(decl)?;
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        provider(format!(
            "API executor needs a tokio runtime to dispatch: {e}"
        ))
    })?;
    tokio::task::block_in_place(|| {
        handle.block_on(dispatch_action(
            action,
            cwd,
            decl,
            default_pool(),
            default_timeout(),
        ))
    })
}

async fn acquire_client(pool: &ApiPool, url: &str) -> ArgoResult<reqwest::Client> {
    let key = base_key(url)?;
    let mut guard = pool.lock().await;
    if let Some(client) = guard.get(&key) {
        return Ok(client.clone());
    }
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            "argo-api-action-executor/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| provider(format!("API action HTTP client: {e}")))?;
    guard.insert(key, client.clone());
    Ok(client)
}

fn build_request(
    client: &reqwest::Client,
    method: Method,
    url: &str,
    binding: &ApiBinding,
    action: &ActionRecord,
) -> ArgoResult<reqwest::RequestBuilder> {
    let mut request = client.request(method.clone(), url);
    for (name, value) in &binding.headers {
        request = request.header(name, render_string(value, action)?);
    }
    if method == Method::GET || method == Method::DELETE {
        return Ok(request);
    }
    if let Some(template) = &binding.body_template {
        let rendered = render_string(template, action)?;
        if let Ok(json_body) = serde_json::from_str::<Value>(&rendered) {
            Ok(request.json(&json_body))
        } else {
            Ok(request.body(rendered))
        }
    } else {
        Ok(request.json(&substitute_args(&action.payload, action)))
    }
}

fn parse_binding(decl: &ActionDeclaration) -> ArgoResult<ApiBinding> {
    let raw = decl.executor.api_binding().ok_or_else(|| {
        provider("API executor missing `executor.binding` with `url`".to_string())
    })?;
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        parse_binding_object(trimmed)
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Ok(ApiBinding {
            url: trimmed.to_string(),
            method: Method::POST,
            headers: HashMap::new(),
            body_template: None,
        })
    } else {
        Err(provider(format!(
            "API executor binding key '{trimmed}' is not yet implemented; \
             executor `api` now supports a direct URL or JSON object string with `url`"
        )))
    }
}

fn parse_binding_object(raw: &str) -> ArgoResult<ApiBinding> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|e| provider(format!("API executor binding JSON parse failed: {e}")))?;
    let url = value
        .get("url")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| provider("API executor binding missing `url`".to_string()))?
        .to_string();
    let method = value
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("POST")
        .parse::<Method>()
        .map_err(|e| provider(format!("API executor binding has invalid method: {e}")))?;
    Ok(ApiBinding {
        url,
        method,
        headers: parse_headers(value.get("headers"))?,
        body_template: value
            .get("body_template")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_headers(value: Option<&Value>) -> ArgoResult<HashMap<String, String>> {
    let Some(Value::Object(map)) = value else {
        return Ok(HashMap::new());
    };
    let mut headers = HashMap::new();
    for (name, value) in map {
        let Some(text) = value.as_str() else {
            return Err(provider(format!(
                "API executor header '{name}' must be a string"
            )));
        };
        headers.insert(name.clone(), text.to_string());
    }
    Ok(headers)
}

fn render_string(template: &str, action: &ActionRecord) -> ArgoResult<String> {
    let (protected, secrets) = protect_secret_placeholders(template);
    let rendered = substitute_args(&Value::String(protected), action);
    let Some(text) = rendered.as_str() else {
        return Ok(template.to_string());
    };
    render_secret_tokens(text, &secrets)
}

fn protect_secret_placeholders(body: &str) -> (String, Vec<String>) {
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
            out.push_str(&format!("@@ARGO_SECRET_{idx}@@"));
        } else {
            out.push_str(&body[open..close + 2]);
        }
        cursor = close + 2;
    }
    out.push_str(&body[cursor..]);
    (out, secrets)
}

fn render_secret_tokens(body: &str, secrets: &[String]) -> ArgoResult<String> {
    let mut out = body.to_string();
    for (idx, secret) in secrets.iter().enumerate() {
        let value = argo_config::cred::get(secret)
            .map_err(|_| provider(format!("API executor missing secret '{secret}'")))?;
        out = out.replace(&format!("@@ARGO_SECRET_{idx}@@"), &value);
    }
    Ok(out)
}

fn base_key(url: &str) -> ArgoResult<String> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|e| provider(format!("API executor invalid url '{url}': {e}")))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| provider(format!("API executor url has no host: {url}")))?;
    let port = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
    Ok(format!("{}://{host}{port}", parsed.scheme()))
}

fn parse_body(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
}

fn body_excerpt(text: &str) -> String {
    const MAX: usize = 500;
    if text.chars().count() <= MAX {
        text.to_string()
    } else {
        format!("{}...", text.chars().take(MAX).collect::<String>())
    }
}

fn timeout_error(action: &ActionRecord, timeout: Duration) -> ArgoError {
    provider(format!(
        "API action '{}' exceeded wallclock budget {}ms",
        action.action_type,
        timeout.as_millis()
    ))
}

fn provider(msg: String) -> ArgoError {
    ArgoError::Provider(msg)
}

#[cfg(test)]
#[path = "worker_actions_executor_api_tests.rs"]
mod tests;
