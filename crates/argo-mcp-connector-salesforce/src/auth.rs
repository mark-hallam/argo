//! OAuth client-credentials helpers for Salesforce.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Connected App client id env var.
pub const SF_CLIENT_ID_ENV: &str = "SF_CLIENT_ID";
/// Connected App client secret env var.
pub const SF_CLIENT_SECRET_ENV: &str = "SF_CLIENT_SECRET";
/// Salesforce instance URL env var, e.g. `https://example.my.salesforce.com`.
pub const SF_INSTANCE_URL_ENV: &str = "SF_INSTANCE_URL";
/// Env var that names the tenant whose cached OAuth token this
/// connector subprocess should read/write. Spawners (the
/// `McpConnectionPool` in `argo-agent-runner`) set this when launching
/// the connector under a specific tenant context; absence falls back to
/// the workspace-stable tenant id so single-tenant deployments still
/// get a stable, isolated key.
pub const TENANT_ID_ENV: &str = "ARGO_MCP_TENANT_ID";

/// OAuth client credentials config.
#[derive(Debug, Clone)]
pub struct SalesforceConfig {
    /// Connected App client id.
    pub client_id: String,
    /// Connected App client secret.
    pub client_secret: String,
    /// Salesforce instance URL.
    pub instance_url: String,
}

/// Cached bearer token and resolved instance URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesforceToken {
    /// OAuth bearer access token.
    pub access_token: String,
    /// Salesforce instance URL returned by OAuth, or the configured URL.
    pub instance_url: String,
}

impl SalesforceConfig {
    /// Build config from `SF_CLIENT_ID`, `SF_CLIENT_SECRET`, and `SF_INSTANCE_URL`.
    pub fn from_env() -> Result<Self, String> {
        let client_id = required_env(SF_CLIENT_ID_ENV)?;
        let client_secret = required_env(SF_CLIENT_SECRET_ENV)?;
        let instance_url = required_env(SF_INSTANCE_URL_ENV)?;
        Ok(Self {
            client_id,
            client_secret,
            instance_url: instance_url.trim_end_matches('/').to_string(),
        })
    }
}

/// Return a cached token when present, otherwise refresh and persist one.
pub async fn access_token_from_env() -> Result<SalesforceToken, String> {
    let cfg = SalesforceConfig::from_env()?;
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let tenant_id = cache_tenant_id(&cwd);
    if let Some(token) = read_cached_token(&cwd, &tenant_id)? {
        if !token.access_token.trim().is_empty() {
            return Ok(token);
        }
    }
    refresh_and_cache(&cfg, &cwd, &tenant_id).await
}

/// Refresh a token using client credentials and persist it under cwd
/// keyed by tenant.
pub async fn refresh_and_cache(
    cfg: &SalesforceConfig,
    cwd: &Path,
    tenant_id: &str,
) -> Result<SalesforceToken, String> {
    let token = refresh(cfg).await?;
    write_cached_token(cwd, tenant_id, &token)?;
    Ok(token)
}

async fn refresh(cfg: &SalesforceConfig) -> Result<SalesforceToken, String> {
    let http = reqwest::Client::builder()
        .user_agent(concat!(
            "argo-mcp-connector-salesforce/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| format!("salesforce oauth client: {e}"))?;
    let url = format!("{}/services/oauth2/token", cfg.instance_url);
    let resp = http
        .post(&url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("salesforce oauth token: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("salesforce oauth token: read response: {e}"))?;
    let parsed: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("salesforce oauth token: decode response: {e}: {text}"))?;
    if !status.is_success() {
        return Err(format!("salesforce oauth token: HTTP {status}: {parsed}"));
    }
    let access_token = parsed
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| "salesforce oauth token: missing access_token".to_string())?;
    let instance_url = parsed
        .get("instance_url")
        .and_then(|v| v.as_str())
        .unwrap_or(&cfg.instance_url);
    Ok(SalesforceToken {
        access_token: access_token.to_string(),
        instance_url: instance_url.trim_end_matches('/').to_string(),
    })
}

fn read_cached_token(cwd: &Path, tenant_id: &str) -> Result<Option<SalesforceToken>, String> {
    migrate_legacy_cache_if_present(cwd, tenant_id)?;
    let path = cache_path(cwd, tenant_id);
    if !path.exists() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| format!("read Salesforce token cache {}: {e}", path.display()))?;
    serde_json::from_str(&body)
        .map(Some)
        .map_err(|e| format!("parse Salesforce token cache {}: {e}", path.display()))
}

fn write_cached_token(cwd: &Path, tenant_id: &str, token: &SalesforceToken) -> Result<(), String> {
    let path = cache_path(cwd, tenant_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "create Salesforce token cache dir {}: {e}",
                parent.display()
            )
        })?;
    }
    let body = serde_json::to_string_pretty(token)
        .map_err(|e| format!("serialize Salesforce token cache: {e}"))?;
    std::fs::write(&path, body)
        .map_err(|e| format!("write Salesforce token cache {}: {e}", path.display()))
}

/// Tenant-keyed cache path under `<cwd>/.argo/connectors/<tenant_id>/salesforce/token.json`.
fn cache_path(cwd: &Path, tenant_id: &str) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join(tenant_id)
        .join("salesforce")
        .join("token.json")
}

/// Legacy (pre-2026-05-23) cwd-shared path that mixed tenants.
fn legacy_cache_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join("salesforce")
        .join("token.json")
}

/// Quarantine destination for the legacy file on first read.
fn legacy_quarantine_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join("legacy")
        .join("salesforce")
        .join("token.json")
}

/// Move a pre-tenant-keyed cache file out of the shared path the first
/// time we see one. Operator re-authenticates the next call so this
/// tenant gets its own token. Don't auto-delete (`legacy/` preserves
/// the file for operator recovery).
fn migrate_legacy_cache_if_present(cwd: &Path, tenant_id: &str) -> Result<(), String> {
    let legacy = legacy_cache_path(cwd);
    if !legacy.exists() {
        return Ok(());
    }
    let dest = legacy_quarantine_path(cwd);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Salesforce token cache legacy mkdir: {e}"))?;
    }
    std::fs::rename(&legacy, &dest).map_err(|e| {
        format!(
            "quarantine legacy Salesforce token cache {} -> {}: {e}",
            legacy.display(),
            dest.display()
        )
    })?;
    tracing::warn!(
        target: "argo_mcp_connector_salesforce",
        tenant_id = tenant_id,
        legacy = %legacy.display(),
        quarantined_to = %dest.display(),
        "moved pre-tenant-keyed Salesforce OAuth token cache to legacy/; re-authenticate to refresh",
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

fn required_env(name: &str) -> Result<String, String> {
    // tenant-unscoped: MCP connector auth init pre-dates per-tenant scoping; follow-up: thread tenant_id through McpClientPool keying
    let value = argo_config::cred::get_unscoped(name).map_err(|_| format!("missing {name}"))?;
    if value.trim().is_empty() {
        return Err(format!("missing {name}"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn missing_config_is_clear() {
        std::env::remove_var(SF_CLIENT_ID_ENV);
        let err = SalesforceConfig::from_env().unwrap_err();
        assert!(err.contains(SF_CLIENT_ID_ENV));
    }

    #[tokio::test]
    async fn refresh_posts_client_credentials_and_caches() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("POST /services/oauth2/token"));
            let body = r#"{"access_token":"tok_123","instance_url":"http://sf.test"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let cwd = unique_temp_dir("sf-auth");
        let cfg = SalesforceConfig {
            client_id: "cid".into(),
            client_secret: "secret".into(),
            instance_url: format!("http://{addr}"),
        };
        let tenant_id = "tenant-a";
        let token = refresh_and_cache(&cfg, &cwd, tenant_id).await.unwrap();
        assert_eq!(token.access_token, "tok_123");
        assert!(cache_path(&cwd, tenant_id).exists());
        let cached = read_cached_token(&cwd, tenant_id).unwrap().unwrap();
        assert_eq!(cached.instance_url, "http://sf.test");
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{prefix}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn token_with(access: &str) -> SalesforceToken {
        SalesforceToken {
            access_token: access.into(),
            instance_url: "https://sf.test".into(),
        }
    }

    // Cross-tenant negative: tenant_b must NEVER read tenant_a's cached
    // token. Pre-fix this returned `tok_a` for both reads because both
    // tenants shared `<cwd>/.argo/connectors/salesforce/token.json`.
    #[test]
    fn cross_tenant_token_isolation() {
        let cwd = unique_temp_dir("sf-isolation");
        write_cached_token(&cwd, "tenant-a", &token_with("tok_a")).unwrap();
        write_cached_token(&cwd, "tenant-b", &token_with("tok_b")).unwrap();

        let a = read_cached_token(&cwd, "tenant-a").unwrap().unwrap();
        let b = read_cached_token(&cwd, "tenant-b").unwrap().unwrap();
        assert_eq!(a.access_token, "tok_a");
        assert_eq!(b.access_token, "tok_b");

        let path_a = cache_path(&cwd, "tenant-a");
        let path_b = cache_path(&cwd, "tenant-b");
        assert_ne!(path_a, path_b);
        assert!(path_a.exists());
        assert!(path_b.exists());
    }

    // Migration on first read: legacy cwd-shared file moves to legacy/
    // and read returns None so the caller refreshes a fresh tenant-keyed
    // token rather than serving the shared one.
    #[test]
    fn legacy_cache_is_quarantined_on_first_read() {
        let cwd = unique_temp_dir("sf-legacy");
        let legacy = legacy_cache_path(&cwd);
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let leaked = token_with("leaked");
        std::fs::write(&legacy, serde_json::to_string(&leaked).unwrap()).unwrap();
        assert!(legacy.exists());

        let result = read_cached_token(&cwd, "tenant-a").unwrap();
        assert!(
            result.is_none(),
            "legacy token must NOT be served to tenant-a",
        );
        assert!(!legacy.exists(), "legacy file should be moved");
        assert!(
            legacy_quarantine_path(&cwd).exists(),
            "legacy file should land under legacy/",
        );
    }
}
