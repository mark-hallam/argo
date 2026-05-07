//! `argo connector authorize` plus shared connector auth helpers.

use crate::cli::ConnectorCmd;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind { OAuth, Token }

#[derive(Clone, Copy, Debug)]
pub struct OAuthSpec { pub client_id_key: &'static str, pub client_secret_key: &'static str, pub refresh_key: Option<&'static str>, pub instance_url_key: Option<&'static str>, pub auth_url: &'static str, pub token_url: &'static str, pub scopes: &'static [&'static str] }

#[derive(Clone, Copy, Debug)]
pub struct ConnectorSpec { pub kind: &'static str, pub auth: AuthKind, pub token_key: &'static str, pub base_url_key: Option<&'static str>, pub default_base_url: &'static str, pub oauth: Option<OAuthSpec> }

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthState { pub kind: String, pub auth_type: AuthKind, pub credential_keys: Vec<String>, pub authorized_at_unix: u64 }

#[derive(Debug, Serialize, Deserialize)]
struct OAuthTokenCache { access_token: String, #[serde(skip_serializing_if = "Option::is_none")] refresh_token: Option<String>, #[serde(skip_serializing_if = "Option::is_none")] expires_at_unix: Option<u64>, #[serde(skip_serializing_if = "Option::is_none")] expires_at: Option<i64>, #[serde(skip_serializing_if = "Option::is_none")] instance_url: Option<String>, #[serde(skip_serializing_if = "Option::is_none")] token_type: Option<String> }

pub const CONNECTORS: &[ConnectorSpec] = &[
    oauth("gmail", "GMAIL_ACCESS_TOKEN", "GMAIL_BASE_URL", "https://gmail.googleapis.com", OAuthSpec { client_id_key: "ARGO_GMAIL_CLIENT_ID", client_secret_key: "ARGO_GMAIL_CLIENT_SECRET", refresh_key: Some("ARGO_GMAIL_REFRESH_TOKEN"), instance_url_key: None, auth_url: "https://accounts.google.com/o/oauth2/v2/auth", token_url: "https://oauth2.googleapis.com/token", scopes: &["https://www.googleapis.com/auth/gmail.modify"] }),
    oauth("gcal", "GCAL_ACCESS_TOKEN", "GCAL_BASE_URL", "https://www.googleapis.com/calendar/v3", OAuthSpec { client_id_key: "ARGO_GCAL_CLIENT_ID", client_secret_key: "ARGO_GCAL_CLIENT_SECRET", refresh_key: Some("ARGO_GCAL_REFRESH_TOKEN"), instance_url_key: None, auth_url: "https://accounts.google.com/o/oauth2/v2/auth", token_url: "https://oauth2.googleapis.com/token", scopes: &["https://www.googleapis.com/auth/calendar"] }),
    oauth("hubspot", "HUBSPOT_PRIVATE_APP_TOKEN", "HUBSPOT_BASE_URL", "https://api.hubapi.com", OAuthSpec { client_id_key: "HUBSPOT_CLIENT_ID", client_secret_key: "HUBSPOT_CLIENT_SECRET", refresh_key: Some("HUBSPOT_REFRESH_TOKEN"), instance_url_key: None, auth_url: "https://app.hubspot.com/oauth/authorize", token_url: "https://api.hubapi.com/oauth/v1/token", scopes: &["crm.objects.contacts.read"] }),
    oauth("notion", "NOTION_INTEGRATION_TOKEN", "NOTION_BASE_URL", "https://api.notion.com/v1", OAuthSpec { client_id_key: "NOTION_CLIENT_ID", client_secret_key: "NOTION_CLIENT_SECRET", refresh_key: Some("NOTION_REFRESH_TOKEN"), instance_url_key: None, auth_url: "https://api.notion.com/v1/oauth/authorize", token_url: "https://api.notion.com/v1/oauth/token", scopes: &[] }),
    oauth("salesforce", "SF_ACCESS_TOKEN", "SF_INSTANCE_URL", "https://login.salesforce.com", OAuthSpec { client_id_key: "SF_CLIENT_ID", client_secret_key: "SF_CLIENT_SECRET", refresh_key: Some("SF_REFRESH_TOKEN"), instance_url_key: Some("SF_INSTANCE_URL"), auth_url: "https://login.salesforce.com/services/oauth2/authorize", token_url: "https://login.salesforce.com/services/oauth2/token", scopes: &["api", "refresh_token"] }),
    token("slack", "SLACK_BOT_TOKEN", "SLACK_API_BASE_URL", "https://slack.com/api"), token("jira", "JIRA_API_TOKEN", "JIRA_BASE_URL", "https://example.atlassian.net"), token("asana", "ASANA_PAT", "ASANA_BASE_URL", "https://app.asana.com/api/1.0"), token("linear", "LINEAR_API_KEY", "LINEAR_BASE_URL", "https://api.linear.app/graphql"), token("monday", "MONDAY_API_TOKEN", "MONDAY_BASE_URL", "https://api.monday.com/v2"), token("pagerduty", "PAGERDUTY_API_KEY", "PAGERDUTY_BASE_URL", "https://api.pagerduty.com"),
    token("fireflies", "FIREFLIES_API_KEY", "FIREFLIES_BASE_URL", "https://api.fireflies.ai/graphql"), token("airtable", "AIRTABLE_PAT", "AIRTABLE_BASE_URL", "https://api.airtable.com/v0"), token("stripe", "STRIPE_API_KEY", "STRIPE_BASE_URL", "https://api.stripe.com"), token("zendesk", "ZENDESK_API_TOKEN", "ZENDESK_BASE_URL", "https://example.zendesk.com"), token("clickup", "CLICKUP_TOKEN", "CLICKUP_BASE_URL", "https://api.clickup.com/api/v2"), token("discord", "DISCORD_BOT_TOKEN", "DISCORD_BASE_URL", "https://discord.com/api/v10"),
];
const fn oauth(kind: &'static str, token_key: &'static str, base_url_key: &'static str, default_base_url: &'static str, oauth: OAuthSpec) -> ConnectorSpec { ConnectorSpec { kind, auth: AuthKind::OAuth, token_key, base_url_key: Some(base_url_key), default_base_url, oauth: Some(oauth) } }
const fn token(kind: &'static str, token_key: &'static str, base_url_key: &'static str, default_base_url: &'static str) -> ConnectorSpec { ConnectorSpec { kind, auth: AuthKind::Token, token_key, base_url_key: Some(base_url_key), default_base_url, oauth: None } }

pub async fn run(cmd: ConnectorCmd) -> ArgoResult<()> { match cmd { ConnectorCmd::Authorize { kind, token, client_id, client_secret, scopes, redirect_port, no_browser, auth_url, token_url, json } => authorize(&kind, token, OAuthArgs { client_id, client_secret, scopes, redirect_port, no_browser, auth_url, token_url }, json).await, ConnectorCmd::ListStatus { json } => crate::handlers::connector_list_status::run(json).await, ConnectorCmd::Revoke { kind, json } => crate::handlers::connector_revoke::run(&kind, json).await, ConnectorCmd::Test { kind, json } => crate::handlers::connector_test::run(&kind, json).await } }

pub struct OAuthArgs { client_id: Option<String>, client_secret: Option<String>, scopes: Vec<String>, redirect_port: u16, no_browser: bool, auth_url: Option<String>, token_url: Option<String> }

async fn authorize(kind: &str, token: Option<String>, oauth_args: OAuthArgs, json_out: bool) -> ArgoResult<()> { let spec = spec_for(kind)?; match spec.auth { AuthKind::Token => authorize_token(spec, token, json_out).await, AuthKind::OAuth => authorize_oauth(spec, oauth_args, json_out).await } }

async fn authorize_token(spec: &'static ConnectorSpec, token: Option<String>, json_out: bool) -> ArgoResult<()> {
    let workspace = workspace_dir()?; let token = match token { Some(token) => token, None => read_hidden_value(&format!("{} token", spec.kind))? };
    if token.trim().is_empty() { return Err(provider_err("token must not be empty")); }
    let outcome = crate::handlers::connector_test::validate_with_token(spec, &token).await?;
    argo_config::cred::set(&workspace, spec.token_key, &token)?; write_auth_state(&workspace, spec, &[spec.token_key])?;
    print_result("connector_authorize", json!({"kind": spec.kind, "auth_type": spec.auth, "validated": outcome.ok, "credential_key": spec.token_key}), json_out)
}

async fn authorize_oauth(spec: &'static ConnectorSpec, args: OAuthArgs, json_out: bool) -> ArgoResult<()> {
    let workspace = workspace_dir()?; let oauth = spec.oauth.ok_or_else(|| provider_err("missing OAuth spec"))?;
    let client_id = resolve_arg_or_credential(&workspace, args.client_id, oauth.client_id_key)?; let client_secret = resolve_arg_or_credential(&workspace, args.client_secret, oauth.client_secret_key)?;
    let listener = TcpListener::bind(("127.0.0.1", args.redirect_port)).map_err(|e| provider_err(format!("bind OAuth callback: {e}")))?;
    let port = listener.local_addr().map_err(|e| provider_err(format!("callback addr: {e}")))?.port(); let redirect_uri = format!("http://127.0.0.1:{port}/callback"); let state = new_state();
    let scopes = if args.scopes.is_empty() { oauth.scopes.iter().map(|s| (*s).to_string()).collect() } else { args.scopes };
    let auth_url = build_authorize_url(args.auth_url.as_deref().unwrap_or(oauth.auth_url), &client_id, &redirect_uri, &state, &scopes);
    if args.no_browser { eprintln!("open this URL to authorize {}: {auth_url}", spec.kind); } else { open_browser(&auth_url)?; }
    let code = capture_oauth_code(listener, state).await?; let token = exchange_code(args.token_url.as_deref().unwrap_or(oauth.token_url), &client_id, &client_secret, &redirect_uri, &code).await?;
    persist_oauth(&workspace, spec, oauth, &client_id, &client_secret, &token)?;
    print_result("connector_authorize", json!({"kind": spec.kind, "auth_type": spec.auth, "credential_key": spec.token_key, "token_cache": cache_path(&workspace, spec)}), json_out)
}

async fn exchange_code(token_url: &str, client_id: &str, client_secret: &str, redirect_uri: &str, code: &str) -> ArgoResult<Value> {
    let resp = reqwest::Client::new().post(token_url).form(&[("grant_type", "authorization_code"), ("code", code), ("redirect_uri", redirect_uri), ("client_id", client_id), ("client_secret", client_secret)]).send().await.map_err(|e| provider_err(format!("OAuth token exchange: {e}")))?;
    let status = resp.status(); let text = resp.text().await.map_err(|e| provider_err(format!("OAuth token response: {e}")))?; let parsed: Value = serde_json::from_str(&text).map_err(|e| provider_err(format!("OAuth token JSON: {e}: {text}")))?;
    if !status.is_success() { return Err(provider_err(format!("OAuth token exchange HTTP {status}: {parsed}"))); } Ok(parsed)
}

fn persist_oauth(workspace: &Path, spec: &ConnectorSpec, oauth: OAuthSpec, client_id: &str, client_secret: &str, token: &Value) -> ArgoResult<()> {
    let access_token = value_str(token, "access_token")?; let refresh_token = token.get("refresh_token").and_then(Value::as_str).map(str::to_string); let expires_at = now_secs() + token.get("expires_in").and_then(Value::as_u64).unwrap_or(3600).saturating_sub(60); let instance_url = token.get("instance_url").and_then(Value::as_str).map(|s| s.trim_end_matches('/').to_string());
    argo_config::cred::set(workspace, spec.token_key, &access_token)?; argo_config::cred::set(workspace, oauth.client_id_key, client_id)?; argo_config::cred::set(workspace, oauth.client_secret_key, client_secret)?;
    if let (Some(key), Some(value)) = (oauth.refresh_key, refresh_token.as_deref()) { argo_config::cred::set(workspace, key, value)?; }
    if let (Some(key), Some(value)) = (oauth.instance_url_key, instance_url.as_deref()) { argo_config::cred::set(workspace, key, value)?; }
    let cache = OAuthTokenCache { access_token, refresh_token, expires_at_unix: Some(expires_at), expires_at: Some(expires_at as i64), instance_url, token_type: token.get("token_type").and_then(Value::as_str).map(str::to_string) }; write_private_json(&cache_path(workspace, spec), &cache)?;
    let mut keys = vec![spec.token_key, oauth.client_id_key, oauth.client_secret_key];
    if let Some(key) = oauth.refresh_key { keys.push(key); }
    if let Some(key) = oauth.instance_url_key { keys.push(key); }
    write_auth_state(workspace, spec, &keys)
}

pub fn write_auth_state(workspace: &Path, spec: &ConnectorSpec, keys: &[&str]) -> ArgoResult<()> { let state = AuthState { kind: spec.kind.to_string(), auth_type: spec.auth, credential_keys: keys.iter().map(|s| (*s).to_string()).collect(), authorized_at_unix: now_secs() }; write_private_json(&auth_state_path(workspace, spec), &state) }
pub fn spec_for(kind: &str) -> ArgoResult<&'static ConnectorSpec> { CONNECTORS.iter().find(|spec| spec.kind == kind).ok_or_else(|| provider_err(format!("unknown connector '{kind}'"))) }
pub fn workspace_dir() -> ArgoResult<PathBuf> { std::env::current_dir().map_err(|e| provider_err(format!("current_dir: {e}"))) }
pub fn connector_dir(workspace: &Path, spec: &ConnectorSpec) -> PathBuf { workspace.join(".argo").join("connectors").join(spec.kind) }
pub fn auth_state_path(workspace: &Path, spec: &ConnectorSpec) -> PathBuf { connector_dir(workspace, spec).join("auth.json") }
pub fn cache_path(workspace: &Path, spec: &ConnectorSpec) -> PathBuf { let name = match spec.kind { "gmail" | "gcal" => "access_token.json", _ => "token.json" }; connector_dir(workspace, spec).join(name) }
pub fn credential_value(workspace: &Path, key: &str) -> Option<String> { argo_config::cred::get_from_workspace(workspace, key).ok().filter(|v| !v.trim().is_empty()) }
pub fn base_url(workspace: &Path, spec: &ConnectorSpec) -> String { spec.base_url_key.and_then(|key| credential_value(workspace, key)).unwrap_or_else(|| spec.default_base_url.to_string()).trim_end_matches('/').to_string() }
pub fn provider_err(message: impl Into<String>) -> ArgoError { ArgoError::Provider(message.into()) }
pub fn now_secs() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

pub fn read_access_token(workspace: &Path, spec: &ConnectorSpec) -> ArgoResult<String> { if let Some(token) = credential_value(workspace, spec.token_key) { return Ok(token); } let body = std::fs::read_to_string(cache_path(workspace, spec)).map_err(|e| provider_err(format!("read {} token cache: {e}", spec.kind)))?; let parsed: Value = serde_json::from_str(&body).map_err(|e| provider_err(format!("parse {} token cache: {e}", spec.kind)))?; value_str(&parsed, "access_token") }

pub fn print_result(label: &str, payload: Value, json_out: bool) -> ArgoResult<()> { if json_out { let mut envelope = serde_json::Map::new(); envelope.insert("ok".into(), Value::Bool(true)); envelope.insert(label.into(), payload.clone()); println!("{}", serde_json::to_string_pretty(&Value::Object(envelope)).map_err(|e| provider_err(format!("serialize JSON: {e}")))?); } else if let Some(kind) = payload.get("kind").and_then(Value::as_str) { println!("{label}: {kind} ok"); } else { println!("{label}: ok"); } Ok(()) }

fn resolve_arg_or_credential(workspace: &Path, arg: Option<String>, key: &str) -> ArgoResult<String> { arg.filter(|s| !s.trim().is_empty()).or_else(|| credential_value(workspace, key)).ok_or_else(|| provider_err(format!("missing {key}; pass --client-id/--client-secret or run `argo cred set {key}`"))) }
fn value_str(value: &Value, key: &str) -> ArgoResult<String> { value.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(str::to_string).ok_or_else(|| provider_err(format!("missing {key}"))) }

fn write_private_json<T: Serialize>(path: &Path, value: &T) -> ArgoResult<()> { if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| provider_err(format!("create {}: {e}", parent.display())))?; } let body = serde_json::to_vec_pretty(value).map_err(|e| provider_err(format!("serialize {}: {e}", path.display())))?; let mut opts = std::fs::OpenOptions::new(); opts.create(true).truncate(true).write(true); #[cfg(unix)] opts.mode(0o600); let mut file = opts.open(path).map_err(|e| provider_err(format!("open {}: {e}", path.display())))?; file.write_all(&body).map_err(|e| provider_err(format!("write {}: {e}", path.display()))) }

fn build_authorize_url(auth_url: &str, client_id: &str, redirect_uri: &str, state: &str, scopes: &[String]) -> String { let mut url = format!("{auth_url}?response_type=code&client_id={}&redirect_uri={}&state={}", encode(client_id), encode(redirect_uri), encode(state)); if !scopes.is_empty() { url.push_str("&scope="); url.push_str(&encode(&scopes.join(" "))); } url.push_str("&access_type=offline&prompt=consent"); url }
async fn capture_oauth_code(listener: TcpListener, expected_state: String) -> ArgoResult<String> { tokio::task::spawn_blocking(move || capture_oauth_code_blocking(listener, &expected_state)).await.map_err(|e| provider_err(format!("OAuth callback task: {e}")))? }

fn capture_oauth_code_blocking(listener: TcpListener, expected_state: &str) -> ArgoResult<String> {
    let (mut stream, _) = listener.accept().map_err(|e| provider_err(format!("accept OAuth callback: {e}")))?; let mut buf = [0_u8; 4096]; let n = stream.read(&mut buf).map_err(|e| provider_err(format!("read OAuth callback: {e}")))?; let req = String::from_utf8_lossy(&buf[..n]); let first = req.lines().next().ok_or_else(|| provider_err("empty OAuth callback"))?; let target = first.split_whitespace().nth(1).ok_or_else(|| provider_err("invalid OAuth callback"))?;
    let params = parse_query(target); let state = params.iter().find(|(k, _)| k == "state").map(|(_, v)| v.as_str()).unwrap_or(""); let code = params.iter().find(|(k, _)| k == "code").map(|(_, v)| v.clone()).ok_or_else(|| provider_err("OAuth callback missing code"))?;
    let ok = state == expected_state; let body = if ok { "Argo connector authorization captured. You can close this tab." } else { "Argo connector authorization failed: state mismatch." }; let status = if ok { "200 OK" } else { "400 Bad Request" }; let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()); stream.write_all(response.as_bytes()).map_err(|e| provider_err(format!("write OAuth callback response: {e}")))?; if ok { Ok(code) } else { Err(provider_err("OAuth callback state mismatch")) }
}

fn parse_query(target: &str) -> Vec<(String, String)> { let query = target.split_once('?').map(|(_, q)| q).unwrap_or(""); query.split('&').filter(|part| !part.is_empty()).map(|part| { let (key, value) = part.split_once('=').unwrap_or((part, "")); (decode(key), decode(value)) }).collect() }
fn open_browser(url: &str) -> ArgoResult<()> { let status = if cfg!(target_os = "macos") { std::process::Command::new("open").arg(url).status() } else if cfg!(target_os = "windows") { std::process::Command::new("cmd").args(["/C", "start", "", url]).status() } else { std::process::Command::new("xdg-open").arg(url).status() }; match status { Ok(status) if status.success() => Ok(()), Ok(status) => Err(provider_err(format!("browser opener exited with {status}"))), Err(e) => Err(provider_err(format!("open browser: {e}"))) } }

fn read_hidden_value(label: &str) -> ArgoResult<String> { eprint!("value for {label}: "); std::io::stderr().flush().map_err(|e| provider_err(format!("stderr flush: {e}")))?; let echo_disabled = disable_echo(); let mut value = String::new(); let result = std::io::stdin().read_line(&mut value).map_err(|e| provider_err(format!("read token from stdin: {e}"))); if echo_disabled { let _ = enable_echo(); eprintln!(); } result?; Ok(value.trim_end_matches(['\r', '\n']).to_string()) }
#[cfg(unix)] fn disable_echo() -> bool { std::process::Command::new("stty").arg("-echo").status().map(|s| s.success()).unwrap_or(false) }
#[cfg(not(unix))] fn disable_echo() -> bool { false }
#[cfg(unix)] fn enable_echo() -> bool { std::process::Command::new("stty").arg("echo").status().map(|s| s.success()).unwrap_or(false) }
#[cfg(not(unix))] fn enable_echo() -> bool { false }

fn encode(input: &str) -> String { input.bytes().flat_map(|b| match b { b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => vec![b as char], _ => format!("%{b:02X}").chars().collect() }).collect() }
fn decode(input: &str) -> String { let bytes = input.as_bytes(); let mut out = Vec::new(); let mut i = 0; while i < bytes.len() { if bytes[i] == b'%' && i + 2 < bytes.len() { if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) { if let Ok(value) = u8::from_str_radix(hex, 16) { out.push(value); i += 3; continue; } } } out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] }); i += 1; } String::from_utf8_lossy(&out).into_owned() }
fn new_state() -> String { format!("argo-{}-{}", std::process::id(), now_secs()) }

#[cfg(test)]
mod tests { use super::*; use std::io::{Read, Write}; use std::net::TcpStream; use std::thread; #[test] fn spec_lookup_knows_slack() { let spec = spec_for("slack").unwrap(); assert_eq!(spec.auth, AuthKind::Token); assert_eq!(spec.token_key, "SLACK_BOT_TOKEN"); } #[test] fn authorize_url_includes_state_and_scope() { let url = build_authorize_url("https://auth.test", "cid", "http://127.0.0.1/cb", "state one", &["a b".into()]); assert!(url.contains("client_id=cid")); assert!(url.contains("state=state%20one")); assert!(url.contains("scope=a%20b")); } #[test] fn callback_captures_code() { let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap(); let addr = listener.local_addr().unwrap(); thread::spawn(move || { let mut stream = TcpStream::connect(addr).unwrap(); stream.write_all(b"GET /callback?code=c123&state=s1 HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap(); let mut body = String::new(); let _ = stream.read_to_string(&mut body); }); let code = capture_oauth_code_blocking(listener, "s1").unwrap(); assert_eq!(code, "c123"); } }
