//! NotifyChannel connectivity checks for `argo doctor`.

use serde_json::{json, Value};
use std::time::Duration;

const TIMEOUT_SECS: u64 = 5;
const POSTMARK_SERVER_URL: &str = "https://api.postmarkapp.com/server";
const SLACK_AUTH_TEST_URL: &str = "https://slack.com/api/auth.test";
const EXPO_SERVER_INFO_URL: &str = "https://exp.host/--/api/v2/server-info";

struct ProbeSpec<'a> {
    name: &'static str,
    url: String,
    auth: Auth<'a>,
}

enum Auth<'a> {
    Bearer(&'a str),
    Basic { user: &'a str, password: &'a str },
    Header { name: &'static str, value: &'a str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeConfig {
    postmark_token: Option<String>,
    slack_token: Option<String>,
    twilio_sid: Option<String>,
    twilio_token: Option<String>,
    expo_token: Option<String>,
    postmark_url: String,
    slack_url: String,
    twilio_url: Option<String>,
    expo_url: String,
}

impl ProbeConfig {
    fn from_env() -> Self {
        let twilio_sid = env_nonempty("TWILIO_ACCOUNT_SID");
        let twilio_url = twilio_sid
            .as_deref()
            .map(|sid| format!("https://api.twilio.com/2010-04-01/Accounts/{sid}.json"));
        Self {
            postmark_token: env_nonempty("POSTMARK_SERVER_TOKEN"),
            slack_token: env_nonempty("SLACK_APPROVALS_BOT_TOKEN"),
            twilio_sid,
            twilio_token: env_nonempty("TWILIO_AUTH_TOKEN"),
            expo_token: env_nonempty("EXPO_ACCESS_TOKEN"),
            postmark_url: POSTMARK_SERVER_URL.into(),
            slack_url: SLACK_AUTH_TEST_URL.into(),
            twilio_url,
            expo_url: EXPO_SERVER_INFO_URL.into(),
        }
    }
}

/// Probe configured approval channels without sending notifications.
pub(super) async fn collect() -> Value {
    collect_with(ProbeConfig::from_env()).await
}

async fn collect_with(cfg: ProbeConfig) -> Value {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build();
    let Ok(client) = client else {
        return json!({
            "email": check("ERROR", "could not build HTTP client"),
            "slack": check("ERROR", "could not build HTTP client"),
            "sms": check("ERROR", "could not build HTTP client"),
            "push": check("ERROR", "could not build HTTP client"),
        });
    };
    let email = match cfg.postmark_token.as_deref() {
        Some(token) => {
            probe(
                &client,
                ProbeSpec {
                    name: "postmark",
                    url: cfg.postmark_url.clone(),
                    auth: Auth::Header {
                        name: "X-Postmark-Server-Token",
                        value: token,
                    },
                },
            )
            .await
        }
        None => check("MISSING", "POSTMARK_SERVER_TOKEN is not set"),
    };
    let slack = match cfg.slack_token.as_deref() {
        Some(token) => {
            probe(
                &client,
                ProbeSpec {
                    name: "slack",
                    url: cfg.slack_url.clone(),
                    auth: Auth::Bearer(token),
                },
            )
            .await
        }
        None => check("MISSING", "SLACK_APPROVALS_BOT_TOKEN is not set"),
    };
    let sms = match (
        cfg.twilio_sid.as_deref(),
        cfg.twilio_token.as_deref(),
        cfg.twilio_url.as_ref(),
    ) {
        (Some(sid), Some(token), Some(url)) => {
            probe(
                &client,
                ProbeSpec {
                    name: "twilio",
                    url: url.clone(),
                    auth: Auth::Basic {
                        user: sid,
                        password: token,
                    },
                },
            )
            .await
        }
        (None, _, _) => check("MISSING", "TWILIO_ACCOUNT_SID is not set"),
        (_, None, _) => check("MISSING", "TWILIO_AUTH_TOKEN is not set"),
        _ => check("MISSING", "Twilio account endpoint could not be built"),
    };
    let push = match cfg.expo_token.as_deref() {
        Some(token) => {
            probe(
                &client,
                ProbeSpec {
                    name: "expo",
                    url: cfg.expo_url.clone(),
                    auth: Auth::Bearer(token),
                },
            )
            .await
        }
        None => check("MISSING", "EXPO_ACCESS_TOKEN is not set"),
    };
    json!({"email": email, "slack": slack, "sms": sms, "push": push})
}

async fn probe(client: &reqwest::Client, spec: ProbeSpec<'_>) -> Value {
    let mut req = client.get(&spec.url).header("Accept", "application/json");
    req = match spec.auth {
        Auth::Bearer(token) => req.bearer_auth(token),
        Auth::Basic { user, password } => req.basic_auth(user, Some(password)),
        Auth::Header { name, value } => req.header(name, value),
    };
    let resp = match req.send().await {
        Ok(resp) => resp,
        Err(e) => return check("ERROR", format!("{} network failure: {e}", spec.name)),
    };
    let status = resp.status();
    let retry_after = resp
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    if status.as_u16() == 429 {
        let detail = retry_after.as_deref().map_or_else(
            || format!("{} rate-limited", spec.name),
            |v| format!("{} rate-limited; retry-after={v}", spec.name),
        );
        return check("WARN", detail)
            .with("http_status", status.as_u16())
            .with_opt("retry_after", retry_after);
    }
    if status.is_success() {
        return check("OK", format!("{} reachable ({status})", spec.name))
            .with("http_status", status.as_u16());
    }
    check("ERROR", format!("{} returned {status}", spec.name)).with("http_status", status.as_u16())
}

fn env_nonempty(name: &str) -> Option<String> {
    argo_config::cred::get(name).ok().filter(|v| !v.trim().is_empty())
}

fn check(status: impl Into<String>, detail: impl Into<String>) -> Value {
    json!({"status": status.into(), "detail": detail.into()})
}

trait WithField {
    fn with(self, key: &str, value: impl serde::Serialize) -> Self;
    fn with_opt(self, key: &str, value: Option<impl serde::Serialize>) -> Self;
}

impl WithField for Value {
    fn with(mut self, key: &str, value: impl serde::Serialize) -> Self {
        if let Some(obj) = self.as_object_mut() {
            obj.insert(key.to_string(), json!(value));
        }
        self
    }

    fn with_opt(mut self, key: &str, value: Option<impl serde::Serialize>) -> Self {
        if let (Some(obj), Some(value)) = (self.as_object_mut(), value) {
            obj.insert(key.to_string(), json!(value));
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn s(v: &Value) -> &str {
        v["status"].as_str().unwrap()
    }

    async fn server(status: u16, headers: &[(&str, &str)], accepts: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let headers: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        tokio::spawn(async move {
            for _ in 0..accepts {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buf = [0_u8; 2048];
                let _ = socket.read(&mut buf).await.unwrap();
                let reason = match status {
                    200 => "OK",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    _ => "Status",
                };
                let mut response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: 2\r\nContent-Type: application/json\r\n"
                );
                for (k, v) in &headers {
                    response.push_str(&format!("{k}: {v}\r\n"));
                }
                response.push_str("\r\n{}");
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        format!("http://{addr}/health")
    }

    fn full_cfg(url: &str) -> ProbeConfig {
        ProbeConfig {
            postmark_token: Some("postmark-token".into()),
            slack_token: Some("slack-token".into()),
            twilio_sid: Some("AC_test".into()),
            twilio_token: Some("twilio-token".into()),
            expo_token: Some("expo-token".into()),
            postmark_url: url.into(),
            slack_url: url.into(),
            twilio_url: Some(url.into()),
            expo_url: url.into(),
        }
    }

    #[tokio::test]
    async fn missing_config_reports_missing() {
        let url = server(200, &[], 1).await;
        let cfg = ProbeConfig {
            postmark_token: None,
            ..full_cfg(&url)
        };
        let v = collect_with(cfg).await;
        assert_eq!(s(&v["email"]), "MISSING");
        assert_eq!(s(&v["slack"]), "OK");
    }

    #[tokio::test]
    async fn ok_status_reports_ok() {
        let url = server(200, &[], 4).await;
        let v = collect_with(full_cfg(&url)).await;
        assert_eq!(s(&v["email"]), "OK");
        assert_eq!(s(&v["slack"]), "OK");
        assert_eq!(s(&v["sms"]), "OK");
        assert_eq!(s(&v["push"]), "OK");
    }

    #[tokio::test]
    async fn rate_limited_reports_warn_with_retry_after() {
        let url = server(429, &[("Retry-After", "17")], 1).await;
        let v = collect_with(full_cfg(&url)).await;
        assert_eq!(s(&v["email"]), "WARN");
        assert_eq!(v["email"]["retry_after"].as_str(), Some("17"));
    }

    #[tokio::test]
    async fn provider_error_reports_error() {
        let url = server(500, &[], 1).await;
        let v = collect_with(full_cfg(&url)).await;
        assert_eq!(s(&v["email"]), "ERROR");
    }

    #[tokio::test]
    async fn push_missing_token_reports_missing() {
        let url = server(200, &[], 1).await;
        let cfg = ProbeConfig {
            expo_token: None,
            ..full_cfg(&url)
        };
        let v = collect_with(cfg).await;
        assert_eq!(s(&v["push"]), "MISSING");
    }

    #[tokio::test]
    async fn network_failure_reports_error() {
        let cfg = full_cfg("http://127.0.0.1:9/closed");
        let v = collect_with(cfg).await;
        assert_eq!(s(&v["email"]), "ERROR");
    }
}
