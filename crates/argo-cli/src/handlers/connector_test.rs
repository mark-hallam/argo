//! `argo connector test` no-op validation calls.

use super::connector_authorize::{base_url, credential_value, provider_err, read_access_token, spec_for, workspace_dir, ConnectorSpec};
use argo_types::ArgoResult;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct TestOutcome { pub kind: String, pub ok: bool, pub endpoint: String, pub status: u16 }

pub async fn run(kind: &str, json_out: bool) -> ArgoResult<()> {
    let spec = spec_for(kind)?; let workspace = workspace_dir()?; let token = read_access_token(&workspace, spec)?; let outcome = validate_with_token(spec, &token).await?;
    if json_out { println!("{}", serde_json::to_string_pretty(&json!({"ok": true, "connector_test": outcome})).map_err(|e| provider_err(format!("serialize JSON: {e}")))?); } else { println!("connector_test: {} HTTP {}", outcome.kind, outcome.status); }
    Ok(())
}

pub async fn validate_with_token(spec: &ConnectorSpec, token: &str) -> ArgoResult<TestOutcome> {
    let workspace = workspace_dir()?; let base = base_url(&workspace, spec);
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).user_agent(concat!("argo-cli/", env!("CARGO_PKG_VERSION"))).build().map_err(|e| provider_err(format!("connector test client: {e}")))?;
    let (request, endpoint) = build_request(&client, spec, &workspace, &base, token)?;
    let response = request.send().await.map_err(|e| provider_err(format!("{} no-op call: {e}", spec.kind)))?;
    let status = response.status(); let text = response.text().await.unwrap_or_default();
    if !status.is_success() || body_reports_failure(spec.kind, &text) { return Err(provider_err(format!("{} no-op call failed HTTP {status}: {text}", spec.kind))); }
    Ok(TestOutcome { kind: spec.kind.to_string(), ok: true, endpoint, status: status.as_u16() })
}

fn build_request(client: &reqwest::Client, spec: &ConnectorSpec, workspace: &std::path::Path, base: &str, token: &str) -> ArgoResult<(reqwest::RequestBuilder, String)> {
    let endpoint = test_endpoint(spec, workspace, base)?;
    let request = match spec.kind { "slack" => client.post(&endpoint).bearer_auth(token), "jira" => jira_request(client, workspace, &endpoint, token), "asana" | "pagerduty" | "airtable" | "stripe" | "clickup" | "discord" | "gmail" | "gcal" | "hubspot" | "salesforce" => client.get(&endpoint).bearer_auth(token), "linear" | "monday" | "fireflies" => client.post(&endpoint).bearer_auth(token).json(&graphql_probe(spec.kind)), "notion" => client.post(&endpoint).bearer_auth(token).header("Notion-Version", "2022-06-28").json(&json!({"page_size": 1})), "zendesk" => zendesk_request(client, workspace, &endpoint, token), other => return Err(provider_err(format!("unknown connector '{other}'"))) };
    Ok((request, endpoint))
}

fn test_endpoint(spec: &ConnectorSpec, workspace: &std::path::Path, base: &str) -> ArgoResult<String> {
    let url = match spec.kind { "slack" => format!("{base}/auth.test"), "jira" => format!("{base}/rest/api/3/myself"), "asana" => format!("{base}/users/me"), "linear" | "monday" | "fireflies" => base.to_string(), "pagerduty" => format!("{base}/users/me"), "airtable" => format!("{base}/meta/whoami"), "stripe" => format!("{base}/v1/balance"), "zendesk" => format!("{base}/api/v2/users/me.json"), "clickup" => format!("{base}/user"), "discord" => format!("{base}/users/@me"), "gmail" => format!("{base}/gmail/v1/users/me/profile"), "gcal" => format!("{base}/users/me/calendarList?maxResults=1"), "hubspot" => format!("{base}/account-info/v3/details"), "notion" => format!("{base}/search"), "salesforce" => salesforce_endpoint(workspace, base), other => return Err(provider_err(format!("unknown connector '{other}'"))) };
    Ok(url)
}

fn salesforce_endpoint(workspace: &std::path::Path, base: &str) -> String { credential_value(workspace, "SF_INSTANCE_URL").unwrap_or_else(|| base.to_string()).trim_end_matches('/').to_string() + "/services/data/v58.0/limits" }
fn jira_request(client: &reqwest::Client, workspace: &std::path::Path, endpoint: &str, token: &str) -> reqwest::RequestBuilder { if let Some(email) = credential_value(workspace, "JIRA_EMAIL") { client.get(endpoint).basic_auth(email, Some(token.to_string())) } else { client.get(endpoint).bearer_auth(token) } }
fn zendesk_request(client: &reqwest::Client, workspace: &std::path::Path, endpoint: &str, token: &str) -> reqwest::RequestBuilder { if let Some(email) = credential_value(workspace, "ZENDESK_EMAIL") { client.get(endpoint).basic_auth(format!("{email}/token"), Some(token.to_string())) } else { client.get(endpoint).bearer_auth(token) } }
fn graphql_probe(kind: &str) -> Value { match kind { "linear" => json!({"query": "query { viewer { id } }"}), "monday" => json!({"query": "query { me { id } }"}), "fireflies" => json!({"query": "query { users { user_id } }"}), _ => json!({}) } }
fn body_reports_failure(kind: &str, body: &str) -> bool { if kind != "slack" { return false; } serde_json::from_str::<Value>(body).ok().and_then(|v| v.get("ok").and_then(Value::as_bool)).is_some_and(|ok| !ok) }

#[cfg(test)]
mod tests { use super::*; use std::io::{Read, Write}; use std::net::TcpListener; use std::thread; #[test] fn graphql_probe_is_kind_specific() { assert!(graphql_probe("linear")["query"].as_str().unwrap().contains("viewer")); assert!(graphql_probe("monday")["query"].as_str().unwrap().contains("me")); } #[tokio::test] async fn slack_validation_hits_auth_test() { let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap(); let addr = listener.local_addr().unwrap(); thread::spawn(move || { let (mut stream, _) = listener.accept().unwrap(); let mut buf = [0; 2048]; let n = stream.read(&mut buf).unwrap(); let req = String::from_utf8_lossy(&buf[..n]); assert!(req.contains("POST /auth.test")); assert!(req.to_ascii_lowercase().contains("authorization: bearer tok")); let body = r#"{"ok":true}"#; let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()); stream.write_all(resp.as_bytes()).unwrap(); }); let dir = tempfile::tempdir().unwrap(); let old = std::env::current_dir().unwrap(); std::env::set_current_dir(dir.path()).unwrap(); std::env::set_var("SLACK_API_BASE_URL", format!("http://{addr}")); let spec = spec_for("slack").unwrap(); let result = validate_with_token(spec, "tok").await.unwrap(); std::env::remove_var("SLACK_API_BASE_URL"); std::env::set_current_dir(old).unwrap(); assert_eq!(result.status, 200); } }
