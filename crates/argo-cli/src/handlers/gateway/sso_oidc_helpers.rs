//! URL-encoding, HTTP write, and IdP token-exchange helpers used by
//! `sso_oidc.rs`.
//!
//! Extracted under the `codex-split-files-450-500-wave` brief to
//! keep the parent module below the 420-LOC headroom target. The
//! parent re-exports [`url_encode`] and [`query_param`] under their
//! original names so external callers (sibling `sso_saml.rs`) keep
//! their existing import paths.
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::sso_oidc::CallbackOutcome;

pub(super) fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub(super) fn query_param(query: &str, key: &str) -> Option<String> {
    for kv in query.split('&') {
        let (k, v) = kv.split_once('=')?;
        if k == key {
            return Some(percent_decode(v));
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((h << 4) | l);
                    i += 2;
                } else {
                    out.push(b'%');
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub(super) async fn write_simple(
    out: &mut TcpStream,
    status: &str,
    body: &str,
) -> tokio::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    out.write_all(resp.as_bytes()).await?;
    out.shutdown().await
}

pub(super) fn err_outcome(status: &'static str, msg: &str) -> CallbackOutcome {
    CallbackOutcome {
        status,
        body: format!(r#"{{"error":"{msg}"}}"#),
        location: None,
        set_cookies: vec![],
    }
}

pub(super) fn load_cfg() -> Result<argo_config::SsoConfig, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let cfg =
        argo_config::ArgoConfig::load(&cwd.join("argo.toml")).map_err(|e| format!("load: {e}"))?;
    Ok(cfg.sso)
}

pub(super) async fn real_token_exchange(
    url: String,
    form: std::collections::BTreeMap<String, String>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("build client: {e}"))?;
    let body: String = form
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let resp = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("send: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("idp status {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("read: {e}"))
}
