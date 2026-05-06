//! Gmail OAuth refresh-token flow and workspace access-token cache.
use oauth2::basic::BasicClient;
use oauth2::{
    reqwest::async_http_client, AuthUrl, ClientId, ClientSecret, RefreshToken, TokenResponse,
    TokenUrl,
};
use std::path::PathBuf;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const CACHE_PATH: &[&str] = &[".argo", "connectors", "gmail", "access_token.json"];

/// Resolve a Gmail access token from cache or OAuth refresh.
pub async fn access_token_from_env() -> Result<String, String> {
    if let Some(token) = read_cached_token()? {
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
    write_cached_token(&access_token, expires_at)?;
    Ok(access_token)
}

fn required_env(name: &str) -> Result<String, String> {
    let value = std::env::var(name).map_err(|_| format!("missing {name}"))?;
    if value.trim().is_empty() {
        Err(format!("missing {name}"))
    } else {
        Ok(value)
    }
}

fn read_cached_token() -> Result<Option<String>, String> {
    let path = cache_path()?;
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

fn write_cached_token(access_token: &str, expires_at: u64) -> Result<(), String> {
    let path = cache_path()?;
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

fn cache_path() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    Ok(CACHE_PATH.iter().fold(cwd, |p, part| p.join(part)))
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

    #[test]
    fn cache_path_ends_with_expected_shape() {
        let p = cache_path().expect("cache path");
        assert!(p.ends_with(".argo/connectors/gmail/access_token.json"));
    }
}
