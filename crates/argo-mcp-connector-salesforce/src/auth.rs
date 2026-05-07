//! OAuth client-credentials helpers for Salesforce.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Connected App client id env var.
pub const SF_CLIENT_ID_ENV: &str = "SF_CLIENT_ID";
/// Connected App client secret env var.
pub const SF_CLIENT_SECRET_ENV: &str = "SF_CLIENT_SECRET";
/// Salesforce instance URL env var, e.g. `https://example.my.salesforce.com`.
pub const SF_INSTANCE_URL_ENV: &str = "SF_INSTANCE_URL";

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
    if let Some(token) = read_cached_token(&cwd)? {
        if !token.access_token.trim().is_empty() {
            return Ok(token);
        }
    }
    refresh_and_cache(&cfg, &cwd).await
}

/// Refresh a token using client credentials and persist it under cwd.
pub async fn refresh_and_cache(
    cfg: &SalesforceConfig,
    cwd: &Path,
) -> Result<SalesforceToken, String> {
    let token = refresh(cfg).await?;
    write_cached_token(cwd, &token)?;
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

fn read_cached_token(cwd: &Path) -> Result<Option<SalesforceToken>, String> {
    let path = cache_path(cwd);
    if !path.exists() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| format!("read Salesforce token cache {}: {e}", path.display()))?;
    serde_json::from_str(&body)
        .map(Some)
        .map_err(|e| format!("parse Salesforce token cache {}: {e}", path.display()))
}

fn write_cached_token(cwd: &Path, token: &SalesforceToken) -> Result<(), String> {
    let path = cache_path(cwd);
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

fn cache_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo/connectors/salesforce/token.json")
}

fn required_env(name: &str) -> Result<String, String> {
    let value = argo_config::cred::get(name).map_err(|_| format!("missing {name}"))?;
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
        let token = refresh_and_cache(&cfg, &cwd).await.unwrap();
        assert_eq!(token.access_token, "tok_123");
        assert!(cache_path(&cwd).exists());
        let cached = read_cached_token(&cwd).unwrap().unwrap();
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
}
