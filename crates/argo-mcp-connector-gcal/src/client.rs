//! Thin Google Calendar REST client.
use argo_mcp::{McpError, McpResult};
use argo_mcp_connector_shared::{default_http_policy, is_http_retriable, retry_with_policy};
use chrono::Utc;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const DEFAULT_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";
const DEFAULT_CALENDAR_ID: &str = "primary";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
/// Env var that names the tenant whose cached OAuth token this
/// connector subprocess should read/write. Spawners (the
/// `McpConnectionPool` in `argo-agent-runner`) set this when launching
/// the connector under a specific tenant context; absence falls back to
/// the workspace-stable tenant id so single-tenant deployments still
/// get a stable, isolated key.
const TENANT_ID_ENV: &str = "ARGO_MCP_TENANT_ID";

/// Runtime configuration for [`GcalClient`].
#[derive(Debug, Clone)]
pub struct GcalConfig {
    /// OAuth2 access token.
    pub access_token: String,
    /// Calendar id, usually `primary`.
    pub calendar_id: String,
    /// Google Calendar API base URL, overridable for tests.
    pub base_url: String,
}

impl GcalConfig {
    /// Build config from environment variables.
    pub async fn from_env() -> McpResult<Self> {
        let access_token = access_token_from_env_or_cache().await?;
        Ok(Self {
            access_token,
            calendar_id: std::env::var("GCAL_CALENDAR_ID")
                .unwrap_or_else(|_| DEFAULT_CALENDAR_ID.into()),
            base_url: std::env::var("GCAL_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.into()),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenCache {
    access_token: String,
    expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<i64>,
}

/// Google Calendar API client.
#[derive(Debug, Clone)]
pub struct GcalClient {
    http: reqwest::Client,
    config: GcalConfig,
}

impl GcalClient {
    /// Construct a client from explicit config.
    #[must_use]
    pub fn new(config: GcalConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    /// List events in a calendar time range.
    pub async fn list_events(&self, args: &Value) -> McpResult<Value> {
        let calendar_id = optional_str(args, "calendar_id").unwrap_or(&self.config.calendar_id);
        let mut query = vec![
            ("singleEvents", "true".to_string()),
            ("orderBy", "startTime".to_string()),
            (
                "maxResults",
                optional_i64(args, "max_results").unwrap_or(10).to_string(),
            ),
        ];
        if let Some(time_min) = optional_str(args, "time_min") {
            query.push(("timeMin", time_min.to_string()));
        }
        if let Some(time_max) = optional_str(args, "time_max") {
            query.push(("timeMax", time_max.to_string()));
        }
        self.request(
            Method::GET,
            &format!("calendars/{}/events", encode(calendar_id)),
            &query,
            None,
        )
        .await
    }

    /// Create an event.
    pub async fn create_event(&self, args: &Value) -> McpResult<Value> {
        let calendar_id = optional_str(args, "calendar_id").unwrap_or(&self.config.calendar_id);
        let body = event_body(args, true)?;
        self.request(
            Method::POST,
            &format!("calendars/{}/events", encode(calendar_id)),
            &[],
            Some(body),
        )
        .await
    }

    /// Patch an existing event.
    pub async fn update_event(&self, args: &Value) -> McpResult<Value> {
        let calendar_id = optional_str(args, "calendar_id").unwrap_or(&self.config.calendar_id);
        let event_id = required_str(args, "event_id")?;
        let body = event_body(args, false)?;
        self.request(
            Method::PATCH,
            &format!(
                "calendars/{}/events/{}",
                encode(calendar_id),
                encode(event_id)
            ),
            &[],
            Some(body),
        )
        .await
    }

    /// Return the free/busy response for calendars in a range.
    pub async fn freebusy(&self, args: &Value) -> McpResult<Value> {
        let calendar_id = optional_str(args, "calendar_id").unwrap_or(&self.config.calendar_id);
        let body = json!({
            "timeMin": required_str(args, "time_min")?,
            "timeMax": required_str(args, "time_max")?,
            "items": [{"id": calendar_id}],
        });
        self.request(Method::POST, "freeBusy", &[], Some(body))
            .await
    }

    /// Patch the caller's response status on an invite.
    pub async fn respond_to_invite(&self, args: &Value) -> McpResult<Value> {
        let calendar_id = optional_str(args, "calendar_id").unwrap_or(&self.config.calendar_id);
        let event_id = required_str(args, "event_id")?;
        let response = required_str(args, "response_status")?;
        let email = required_str(args, "attendee_email")?;
        let body = json!({"attendees": [{"email": email, "responseStatus": response}]});
        self.request(
            Method::PATCH,
            &format!(
                "calendars/{}/events/{}",
                encode(calendar_id),
                encode(event_id)
            ),
            &[("sendUpdates", "all".to_string())],
            Some(body),
        )
        .await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> McpResult<Value> {
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), path);
        let policy = default_http_policy();
        retry_with_policy(&policy, is_http_retriable, || {
            self.send_once(method.clone(), url.clone(), query, body.clone())
        })
        .await
    }

    async fn send_once(
        &self,
        method: Method,
        url: String,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> McpResult<Value> {
        let mut req = self
            .http
            .request(method, url)
            .bearer_auth(&self.config.access_token)
            .query(query);
        if let Some(body) = body {
            req = req.json(&body);
        }
        let response = req
            .send()
            .await
            .map_err(|e| McpError::Io(format!("gcal request: {e}")))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| McpError::Io(format!("gcal response: {e}")))?;
        let parsed =
            serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({ "raw": text }));
        if !status.is_success() {
            return Err(McpError::Rpc {
                code: i64::from(status.as_u16()),
                message: "gcal api error".into(),
                data: Some(parsed),
            });
        }
        Ok(parsed)
    }
}

pub(crate) fn required_str<'a>(args: &'a Value, key: &str) -> McpResult<&'a str> {
    optional_str(args, key).ok_or_else(|| McpError::Initialize(format!("missing {key}")))
}

pub(crate) fn optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

pub(crate) fn optional_i64(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(Value::as_i64)
}

fn event_body(args: &Value, require_times: bool) -> McpResult<Value> {
    let mut event = serde_json::Map::new();
    if let Some(summary) = optional_str(args, "summary") {
        event.insert("summary".into(), json!(summary));
    }
    if let Some(description) = optional_str(args, "description") {
        event.insert("description".into(), json!(description));
    }
    if let Some(location) = optional_str(args, "location") {
        event.insert("location".into(), json!(location));
    }
    if require_times || optional_str(args, "start").is_some() {
        event.insert(
            "start".into(),
            date_time(required_str(args, "start")?, args),
        );
    }
    if require_times || optional_str(args, "end").is_some() {
        event.insert("end".into(), date_time(required_str(args, "end")?, args));
    }
    if let Some(attendees) = args.get("attendees").and_then(Value::as_array) {
        event.insert("attendees".into(), json!(attendees));
    }
    Ok(Value::Object(event))
}

fn date_time(value: &str, args: &Value) -> Value {
    json!({
        "dateTime": value,
        "timeZone": optional_str(args, "time_zone").unwrap_or("UTC"),
    })
}

fn encode(segment: &str) -> String {
    urlencoding::encode(segment).into_owned()
}

async fn access_token_from_env_or_cache() -> McpResult<String> {
    // tenant-unscoped: MCP connector auth init pre-dates per-tenant scoping; follow-up: thread tenant_id through McpClientPool keying
    if let Ok(token) = argo_config::cred::get_unscoped("GCAL_ACCESS_TOKEN") {
        return Ok(token);
    }
    // tenant-unscoped: MCP connector auth init pre-dates per-tenant scoping; follow-up: thread tenant_id through McpClientPool keying
    if let Ok(token) = argo_config::cred::get_unscoped("GCAL_TEST_TOKEN") {
        return Ok(token);
    }
    let cwd = std::env::current_dir().map_err(|e| McpError::Io(format!("current_dir: {e}")))?;
    let tenant_id = cache_tenant_id(&cwd);
    if let Some(token) = read_cached_access_token(&cwd, &tenant_id)? {
        return Ok(token);
    }
    refresh_access_token(&cwd, &tenant_id).await
}

fn read_cached_access_token(cwd: &Path, tenant_id: &str) -> McpResult<Option<String>> {
    migrate_legacy_cache_if_present(cwd, tenant_id)?;
    let path = token_cache_path(cwd, tenant_id);
    if !path.exists() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| McpError::Io(format!("read {}: {e}", path.display())))?;
    let cache: AccessTokenCache = serde_json::from_str(&body)
        .map_err(|e| McpError::Decode(format!("parse {}: {e}", path.display())))?;
    if cache.expires_at > Utc::now().timestamp() + 60 {
        Ok(Some(cache.access_token))
    } else {
        Ok(None)
    }
}

async fn refresh_access_token(cwd: &Path, tenant_id: &str) -> McpResult<String> {
    let refresh_token = required_env("ARGO_GCAL_REFRESH_TOKEN")?;
    let client_id = required_env("ARGO_GCAL_CLIENT_ID")?;
    let client_secret = required_env("ARGO_GCAL_CLIENT_SECRET")?;
    let response = reqwest::Client::new()
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| McpError::Io(format!("gcal token refresh: {e}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| McpError::Io(format!("gcal token response: {e}")))?;
    if !status.is_success() {
        return Err(McpError::Rpc {
            code: i64::from(status.as_u16()),
            message: "gcal token refresh failed".into(),
            data: serde_json::from_str(&body).ok(),
        });
    }
    let refreshed: RefreshResponse = serde_json::from_str(&body)
        .map_err(|e| McpError::Decode(format!("gcal token JSON: {e}")))?;
    write_cached_access_token(cwd, tenant_id, &refreshed)?;
    Ok(refreshed.access_token)
}

fn write_cached_access_token(cwd: &Path, tenant_id: &str, token: &RefreshResponse) -> McpResult<()> {
    let path = token_cache_path(cwd, tenant_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| McpError::Io(format!("create {}: {e}", parent.display())))?;
    }
    let cache = AccessTokenCache {
        access_token: token.access_token.clone(),
        expires_at: Utc::now().timestamp() + token.expires_in.unwrap_or(3600),
    };
    let body = serde_json::to_string_pretty(&cache)
        .map_err(|e| McpError::Decode(format!("serialize gcal token cache: {e}")))?;
    std::fs::write(&path, body).map_err(|e| McpError::Io(format!("write {}: {e}", path.display())))
}

/// Tenant-keyed cache path under `<cwd>/.argo/connectors/<tenant_id>/gcal/access_token.json`.
fn token_cache_path(cwd: &Path, tenant_id: &str) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join(tenant_id)
        .join("gcal")
        .join("access_token.json")
}

/// Legacy (pre-2026-05-23) cwd-shared path that mixed tenants.
fn legacy_cache_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join("gcal")
        .join("access_token.json")
}

/// Quarantine destination for the legacy file on first read.
fn legacy_quarantine_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join("legacy")
        .join("gcal")
        .join("access_token.json")
}

/// Move a pre-tenant-keyed cache file out of the shared path the first
/// time we see one. Operator re-authenticates the next call so this
/// tenant gets its own token. Don't auto-delete (`legacy/` preserves
/// the file for operator recovery).
fn migrate_legacy_cache_if_present(cwd: &Path, tenant_id: &str) -> McpResult<()> {
    let legacy = legacy_cache_path(cwd);
    if !legacy.exists() {
        return Ok(());
    }
    let dest = legacy_quarantine_path(cwd);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| McpError::Io(format!("create {}: {e}", parent.display())))?;
    }
    if let Err(e) = std::fs::rename(&legacy, &dest) {
        return Err(McpError::Io(format!(
            "quarantine legacy gcal token cache {} -> {}: {e}",
            legacy.display(),
            dest.display()
        )));
    }
    tracing::warn!(
        target: "argo_mcp_connector_gcal",
        tenant_id = tenant_id,
        legacy = %legacy.display(),
        quarantined_to = %dest.display(),
        "moved pre-tenant-keyed gcal OAuth token cache to legacy/; re-authenticate to refresh",
    );
    Ok(())
}

/// Resolve the tenant id this connector subprocess should key its
/// cache under. Prefers the spawner-set `ARGO_MCP_TENANT_ID` env var;
/// falls back to the workspace-stable id from `argo.toml` so a
/// single-tenant deployment still isolates by workspace rather than
/// reusing the legacy shared path.
fn cache_tenant_id(cwd: &Path) -> String {
    if let Ok(id) = std::env::var(TENANT_ID_ENV) {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    argo_config::workspace_tenant_id(cwd)
}

fn required_env(name: &str) -> McpResult<String> {
    // tenant-unscoped: MCP connector auth init pre-dates per-tenant scoping; follow-up: thread tenant_id through McpClientPool keying
    argo_config::cred::get_unscoped(name).map_err(|_| McpError::Initialize(format!("{name} is required")))
}

#[cfg(test)]
#[path = "client_cache_tenant_tests.rs"]
mod cache_tenant_tests;
