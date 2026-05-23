//! Gmail OAuth refresh-token flow and workspace access-token cache.
use oauth2::basic::BasicClient;
use oauth2::{
    reqwest::async_http_client, AuthUrl, ClientId, ClientSecret, RefreshToken, TokenResponse,
    TokenUrl,
};
use std::path::{Path, PathBuf};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
/// Env var that names the tenant whose cached OAuth token this
/// connector subprocess should read/write. Spawners (the
/// `McpConnectionPool` in `argo-agent-runner`) set this when launching
/// the connector under a specific tenant context; absence falls back to
/// the workspace-stable tenant id so single-tenant deployments still
/// get a stable, isolated key.
const TENANT_ID_ENV: &str = "ARGO_MCP_TENANT_ID";

/// Resolve a Gmail access token from cache or OAuth refresh.
pub async fn access_token_from_env() -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    let tenant_id = cache_tenant_id(&cwd);
    if let Some(token) = read_cached_token(&cwd, &tenant_id)? {
        return Ok(token);
    }
    refresh_and_cache().await
}

/// Force a refresh and persist the new access token cache.
pub async fn refresh_and_cache() -> Result<String, String> {
    let client_id = required_env("ARGO_GMAIL_CLIENT_ID")?;
    let client_secret = required_env("ARGO_GMAIL_CLIENT_SECRET")?;
    let refresh_token = required_env("ARGO_GMAIL_REFRESH_TOKEN")?;
    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(AUTH_URL.to_string()).map_err(|e| format!("gmail auth URL: {e}"))?,
        Some(TokenUrl::new(TOKEN_URL.to_string()).map_err(|e| format!("gmail token URL: {e}"))?),
    );
    let token = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token))
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("gmail OAuth refresh failed: {e}"))?;
    let access_token = token.access_token().secret().to_string();
    let expires_at = now_secs()
        + token
            .expires_in()
            .map(|d| d.as_secs())
            .unwrap_or(3600)
            .saturating_sub(60);
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    let tenant_id = cache_tenant_id(&cwd);
    write_cached_token(&cwd, &tenant_id, &access_token, expires_at)?;
    Ok(access_token)
}

fn required_env(name: &str) -> Result<String, String> {
    // tenant-unscoped: MCP connector auth init pre-dates per-tenant scoping; follow-up: thread tenant_id through McpClientPool keying
    let value = argo_config::cred::get_unscoped(name).map_err(|_| format!("missing {name}"))?;
    if value.trim().is_empty() {
        Err(format!("missing {name}"))
    } else {
        Ok(value)
    }
}

fn read_cached_token(cwd: &Path, tenant_id: &str) -> Result<Option<String>, String> {
    migrate_legacy_cache_if_present(cwd, tenant_id)?;
    let path = cache_path(cwd, tenant_id);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("gmail token cache read: {e}")),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("gmail token cache parse: {e}"))?;
    let token = value
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty());
    let expires_at = value.get("expires_at_unix").and_then(|v| v.as_u64());
    match (token, expires_at) {
        (Some(token), Some(expires_at)) if expires_at > now_secs() + 60 => {
            Ok(Some(token.to_string()))
        }
        _ => Ok(None),
    }
}

fn write_cached_token(
    cwd: &Path,
    tenant_id: &str,
    access_token: &str,
    expires_at: u64,
) -> Result<(), String> {
    let path = cache_path(cwd, tenant_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("gmail token cache mkdir: {e}"))?;
    }
    let body = serde_json::json!({
        "access_token": access_token,
        "expires_at_unix": expires_at
    })
    .to_string();
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    use std::io::Write;
    let mut file = opts
        .open(&path)
        .map_err(|e| format!("gmail token cache open: {e}"))?;
    file.write_all(body.as_bytes())
        .map_err(|e| format!("gmail token cache write: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .map_err(|e| format!("gmail token cache chmod: {e}"))?;
    }
    Ok(())
}

/// Tenant-keyed cache path under `<cwd>/.argo/connectors/<tenant_id>/gmail/access_token.json`.
fn cache_path(cwd: &Path, tenant_id: &str) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join(tenant_id)
        .join("gmail")
        .join("access_token.json")
}

/// Legacy (pre-2026-05-23) cwd-shared path that mixed tenants.
fn legacy_cache_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join("gmail")
        .join("access_token.json")
}

/// Quarantine destination for the legacy file on first read.
fn legacy_quarantine_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo")
        .join("connectors")
        .join("legacy")
        .join("gmail")
        .join("access_token.json")
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
            .map_err(|e| format!("gmail token cache legacy mkdir: {e}"))?;
    }
    std::fs::rename(&legacy, &dest).map_err(|e| {
        format!(
            "quarantine legacy gmail token cache {} -> {}: {e}",
            legacy.display(),
            dest.display()
        )
    })?;
    tracing::warn!(
        target: "argo_mcp_connector_gmail",
        tenant_id = tenant_id,
        legacy = %legacy.display(),
        quarantined_to = %dest.display(),
        "moved pre-tenant-keyed gmail OAuth token cache to legacy/; re-authenticate to refresh",
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

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn cache_path_includes_tenant_segment() {
        let p = cache_path(Path::new("/work"), "tenant-a");
        assert!(p.ends_with(".argo/connectors/tenant-a/gmail/access_token.json"));
    }

    #[test]
    fn legacy_cache_path_matches_pre_tenant_shape() {
        let p = legacy_cache_path(Path::new("/work"));
        assert!(p.ends_with(".argo/connectors/gmail/access_token.json"));
    }

    // Cross-tenant negative: tenant_b must NEVER read tenant_a's cached
    // token. Pre-fix this returned `tok_a` for both reads because both
    // tenants shared `<cwd>/.argo/connectors/gmail/access_token.json`.
    #[test]
    fn cross_tenant_token_isolation() {
        let tmp = TempDir::new().expect("tempdir");
        let cwd = tmp.path();
        let future = now_secs() + 3600;

        write_cached_token(cwd, "tenant-a", "tok_a", future).unwrap();
        write_cached_token(cwd, "tenant-b", "tok_b", future).unwrap();

        assert_eq!(
            read_cached_token(cwd, "tenant-a").unwrap(),
            Some("tok_a".to_string())
        );
        assert_eq!(
            read_cached_token(cwd, "tenant-b").unwrap(),
            Some("tok_b".to_string())
        );

        let path_a = cache_path(cwd, "tenant-a");
        let path_b = cache_path(cwd, "tenant-b");
        assert_ne!(path_a, path_b);
        assert!(path_a.exists());
        assert!(path_b.exists());
    }

    // Migration on first read: legacy cwd-shared file moves to legacy/
    // and read returns None so the caller refreshes a fresh tenant-keyed
    // token rather than serving the shared one.
    #[test]
    fn legacy_cache_is_quarantined_on_first_read() {
        let tmp = TempDir::new().expect("tempdir");
        let cwd = tmp.path();
        let legacy = legacy_cache_path(cwd);
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(
            &legacy,
            serde_json::json!({"access_token": "leaked", "expires_at_unix": now_secs() + 3600})
                .to_string(),
        )
        .unwrap();
        assert!(legacy.exists());

        let result = read_cached_token(cwd, "tenant-a").unwrap();
        assert!(
            result.is_none(),
            "legacy token must NOT be served to tenant-a",
        );
        assert!(!legacy.exists(), "legacy file should be moved");
        assert!(
            legacy_quarantine_path(cwd).exists(),
            "legacy file should land under legacy/",
        );
    }
}
