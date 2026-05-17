//! OAuth helpers: authorize-URL builder, code-exchange, loopback callback capture.

use super::connector_authorize_paths::provider_err;
use argo_types::ArgoResult;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;

pub(super) async fn exchange_code(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> ArgoResult<Value> {
    let resp = reqwest::Client::new()
        .post(token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()
        .await
        .map_err(|e| provider_err(format!("OAuth token exchange: {e}")))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| provider_err(format!("OAuth token response: {e}")))?;
    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| provider_err(format!("OAuth token JSON: {e}: {text}")))?;
    if !status.is_success() {
        return Err(provider_err(format!(
            "OAuth token exchange HTTP {status}: {parsed}"
        )));
    }
    Ok(parsed)
}

pub(super) fn build_authorize_url(
    auth_url: &str,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    scopes: &[String],
) -> String {
    let mut url = format!(
        "{auth_url}?response_type=code&client_id={}&redirect_uri={}&state={}",
        encode(client_id),
        encode(redirect_uri),
        encode(state)
    );
    if !scopes.is_empty() {
        url.push_str("&scope=");
        url.push_str(&encode(&scopes.join(" ")));
    }
    url.push_str("&access_type=offline&prompt=consent");
    url
}

pub(super) async fn capture_oauth_code(
    listener: TcpListener,
    expected_state: String,
) -> ArgoResult<String> {
    tokio::task::spawn_blocking(move || capture_oauth_code_blocking(listener, &expected_state))
        .await
        .map_err(|e| provider_err(format!("OAuth callback task: {e}")))?
}

pub(super) fn capture_oauth_code_blocking(
    listener: TcpListener,
    expected_state: &str,
) -> ArgoResult<String> {
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| provider_err(format!("accept OAuth callback: {e}")))?;
    let mut buf = [0_u8; 4096];
    let n = stream
        .read(&mut buf)
        .map_err(|e| provider_err(format!("read OAuth callback: {e}")))?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req
        .lines()
        .next()
        .ok_or_else(|| provider_err("empty OAuth callback"))?;
    let target = first
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| provider_err("invalid OAuth callback"))?;
    let params = parse_query(target);
    let state = params
        .iter()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    let code = params
        .iter()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.clone())
        .ok_or_else(|| provider_err("OAuth callback missing code"))?;
    let ok = state == expected_state;
    let body = if ok {
        "Argo connector authorization captured. You can close this tab."
    } else {
        "Argo connector authorization failed: state mismatch."
    };
    let status = if ok { "200 OK" } else { "400 Bad Request" };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| provider_err(format!("write OAuth callback response: {e}")))?;
    if ok {
        Ok(code)
    } else {
        Err(provider_err("OAuth callback state mismatch"))
    }
}

fn parse_query(target: &str) -> Vec<(String, String)> {
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (decode(key), decode(value))
        })
        .collect()
}

pub(super) fn open_browser(url: &str) -> ArgoResult<()> {
    let status = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()
    } else {
        std::process::Command::new("xdg-open").arg(url).status()
    };
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(provider_err(format!("browser opener exited with {status}"))),
        Err(e) => Err(provider_err(format!("open browser: {e}"))),
    }
}

pub(super) fn new_state() -> String {
    format!(
        "argo-{}-{}",
        std::process::id(),
        super::connector_authorize_paths::now_secs()
    )
}

fn encode(input: &str) -> String {
    input
        .bytes()
        .flat_map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => vec![b as char],
            _ => format!("%{b:02X}").chars().collect(),
        })
        .collect()
}

fn decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    out.push(value);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;

    #[test]
    fn authorize_url_includes_state_and_scope() {
        let url = build_authorize_url(
            "https://auth.test",
            "cid",
            "http://127.0.0.1/cb",
            "state one",
            &["a b".into()],
        );
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("state=state%20one"));
        assert!(url.contains("scope=a%20b"));
    }

    #[test]
    fn callback_captures_code() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream
                .write_all(b"GET /callback?code=c123&state=s1 HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();
            let mut body = String::new();
            let _ = stream.read_to_string(&mut body);
        });
        let code = capture_oauth_code_blocking(listener, "s1").unwrap();
        assert_eq!(code, "c123");
    }
}
