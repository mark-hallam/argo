//! DNS, TLS, and provider-auth probes for `argo doctor`.

use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_HOSTS: [&str; 3] = ["api.anthropic.com", "api.openai.com", "skycot.com"];

struct AuthTarget {
    provider: &'static str,
    host: &'static str,
    env_var: &'static str,
    url: &'static str,
}

const AUTH_TARGETS: [AuthTarget; 2] = [
    AuthTarget {
        provider: "anthropic",
        host: "api.anthropic.com",
        env_var: "ANTHROPIC_API_KEY",
        url: "https://api.anthropic.com/v1/models",
    },
    AuthTarget {
        provider: "openai",
        host: "api.openai.com",
        env_var: "OPENAI_API_KEY",
        url: "https://api.openai.com/v1/models",
    },
];

#[async_trait::async_trait]
trait NetworkProbe {
    async fn dns(&self, host: &str) -> Value;
    async fn tls(&self, host: &str) -> Value;
    async fn auth(&self, cwd: &Path, target: &AuthTarget) -> Value;
}

struct RealProbe;

pub(super) fn skip_from_env() -> bool {
    std::env::var("ARGO_DOCTOR_SKIP_NETWORK")
        .is_ok_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
}

pub(super) async fn collect(cwd: &Path, skip: bool) -> Value {
    collect_with_probe(cwd, skip, &RealProbe).await
}

async fn collect_with_probe(cwd: &Path, skip: bool, probe: &dyn NetworkProbe) -> Value {
    if skip {
        return json!({"skipped": true, "reason": "skip requested", "dns": [], "tls": [], "auth": []});
    }
    let hosts = configured_hosts(cwd);
    let mut dns = Vec::with_capacity(hosts.len());
    let mut tls = Vec::with_capacity(hosts.len());
    for host in &hosts {
        dns.push(probe.dns(host).await);
        tls.push(probe.tls(host).await);
    }
    let mut auth = Vec::with_capacity(AUTH_TARGETS.len());
    for target in &AUTH_TARGETS {
        auth.push(probe.auth(cwd, target).await);
    }
    json!({"skipped": false, "dns": dns, "tls": tls, "auth": auth})
}

pub(super) fn print_text(v: &Value) {
    println!("  Network checks:");
    if v["skipped"].as_bool().unwrap_or(false) {
        println!(
            "    SKIP network             {}",
            v["reason"].as_str().unwrap_or("skip requested")
        );
        return;
    }
    for key in ["dns", "tls", "auth"] {
        for item in v[key].as_array().into_iter().flatten() {
            let status = item["status"].as_str().unwrap_or("FAIL");
            let host = item["host"].as_str().unwrap_or("?");
            let detail = item["detail"].as_str().unwrap_or("");
            println!(
                "    {:<4} {:<3} {:<24} {}",
                key.to_uppercase(),
                status,
                host,
                detail
            );
        }
    }
}

#[async_trait::async_trait]
impl NetworkProbe for RealProbe {
    async fn dns(&self, host: &str) -> Value {
        let started = Instant::now();
        let lookup = tokio::time::timeout(TIMEOUT, tokio::net::lookup_host((host, 443))).await;
        match lookup {
            Ok(Ok(mut addrs)) => match addrs.next() {
                Some(addr) => ok(host, addr.ip().to_string(), started),
                None => fail(host, "no addresses returned", started),
            },
            Ok(Err(e)) => fail(host, format!("resolve: {e}"), started),
            Err(_) => fail(host, "resolve timed out", started),
        }
    }

    async fn tls(&self, host: &str) -> Value {
        let started = Instant::now();
        let client = match reqwest::Client::builder().timeout(TIMEOUT).build() {
            Ok(client) => client,
            Err(e) => return fail(host, format!("client: {e}"), started),
        };
        match client.head(format!("https://{host}/")).send().await {
            Ok(_) => {
                let cert = cert_summary(host)
                    .await
                    .unwrap_or_else(|| "certificate details unavailable".into());
                ok(host, cert, started)
            }
            Err(e) => fail(host, format!("tls: {e}"), started),
        }
    }

    async fn auth(&self, cwd: &Path, target: &AuthTarget) -> Value {
        let Ok(key) = argo_config::cred::get_from_workspace(cwd, target.env_var) else {
            return json!({"provider": target.provider, "host": target.host, "status": "SKIP", "detail": "no key configured"});
        };
        let started = Instant::now();
        let client = match reqwest::Client::builder().timeout(TIMEOUT).build() {
            Ok(client) => client,
            Err(e) => return fail_auth(target, format!("client: {e}"), started),
        };
        let mut req = client.get(target.url);
        if target.provider == "anthropic" {
            req = req
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01");
        } else {
            req = req.bearer_auth(key);
        }
        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                ok_auth(target, resp.status().as_u16(), started)
            }
            Ok(resp) => fail_auth(target, format!("HTTP {}", resp.status().as_u16()), started),
            Err(e) => fail_auth(target, format!("auth: {e}"), started),
        }
    }
}

fn configured_hosts(cwd: &Path) -> Vec<String> {
    let mut hosts: Vec<String> = DEFAULT_HOSTS.iter().map(|s| (*s).to_string()).collect();
    let Ok(body) = std::fs::read_to_string(cwd.join("argo.toml")) else {
        return hosts;
    };
    let Ok(value) = toml::from_str::<toml::Value>(&body) else {
        return hosts;
    };
    let Some(extra) = value
        .get("doctor")
        .and_then(|v| v.get("additional_hosts"))
        .and_then(|v| v.as_array())
    else {
        return hosts;
    };
    for host in extra
        .iter()
        .filter_map(|v| v.as_str())
        .filter(|h| valid_host(h))
    {
        if !hosts.iter().any(|existing| existing == host) {
            hosts.push(host.to_string());
        }
    }
    hosts
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_alphanumeric())
                && label
                    .bytes()
                    .last()
                    .is_some_and(|b| b.is_ascii_alphanumeric())
        })
}

async fn cert_summary(host: &str) -> Option<String> {
    if !valid_host(host) {
        return None;
    }
    let cmd = format!(
        "printf '' | openssl s_client -servername {host} -connect {host}:443 2>/dev/null | openssl x509 -noout -enddate -fingerprint -sha256"
    );
    let out = tokio::time::timeout(
        TIMEOUT,
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    let mut expiry = None;
    let mut fingerprint = None;
    for line in raw.lines() {
        if let Some(value) = line.strip_prefix("notAfter=") {
            expiry = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("sha256 Fingerprint=") {
            fingerprint = Some(value.replace(':', "").to_ascii_lowercase());
        }
    }
    match (expiry, fingerprint) {
        (Some(expiry), Some(fingerprint)) => {
            Some(format!("expires {expiry}, sha256 {fingerprint}"))
        }
        (Some(expiry), None) => Some(format!("expires {expiry}")),
        (None, Some(fingerprint)) => Some(format!("sha256 {fingerprint}")),
        (None, None) => None,
    }
}

fn ok(host: &str, detail: String, started: Instant) -> Value {
    json!({"host": host, "status": "OK", "detail": detail, "latency_ms": started.elapsed().as_millis()})
}
fn fail(host: &str, detail: impl Into<String>, started: Instant) -> Value {
    json!({"host": host, "status": "FAIL", "detail": detail.into(), "latency_ms": started.elapsed().as_millis()})
}
fn ok_auth(target: &AuthTarget, code: u16, started: Instant) -> Value {
    json!({"provider": target.provider, "host": target.host, "status": "OK", "status_code": code, "detail": format!("HTTP {code}"), "latency_ms": started.elapsed().as_millis()})
}
fn fail_auth(target: &AuthTarget, detail: impl Into<String>, started: Instant) -> Value {
    json!({"provider": target.provider, "host": target.host, "status": "FAIL", "detail": detail.into(), "latency_ms": started.elapsed().as_millis()})
}

#[cfg(test)]
#[path = "doctor_network_tests.rs"]
mod doctor_network_tests;
