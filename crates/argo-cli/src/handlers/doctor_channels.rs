//! NotifyChannel connectivity checks for `argo doctor`.

use serde_json::{json, Value};
use std::time::Duration;

#[path = "doctor_channels_kinds.rs"]
mod kinds;

const TIMEOUT_SECS: u64 = 5;
const POSTMARK_SERVER_URL: &str = "https://api.postmarkapp.com/server";
const SLACK_AUTH_TEST_URL: &str = "https://slack.com/api/auth.test";
const DISCORD_AUTH_TEST_URL: &str = "https://discord.com/api/v10/users/@me";
const EXPO_SERVER_INFO_URL: &str = "https://exp.host/--/api/v2/server-info";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProbeConfig {
    pub(super) postmark_token: Option<String>,
    pub(super) slack_token: Option<String>,
    pub(super) discord_token: Option<String>,
    pub(super) twilio_sid: Option<String>,
    pub(super) twilio_token: Option<String>,
    pub(super) expo_token: Option<String>,
    pub(super) bluebubbles_url: Option<String>,
    pub(super) bluebubbles_password: Option<String>,
    pub(super) voice_from_number: Option<String>,
    pub(super) postmark_url: String,
    pub(super) slack_url: String,
    pub(super) discord_url: String,
    pub(super) twilio_url: Option<String>,
    pub(super) expo_url: String,
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
            discord_token: env_nonempty("DISCORD_BOT_TOKEN"),
            twilio_sid,
            twilio_token: env_nonempty("TWILIO_AUTH_TOKEN"),
            expo_token: env_nonempty("EXPO_ACCESS_TOKEN"),
            bluebubbles_url: env_nonempty("BLUE_BUBBLES_URL"),
            bluebubbles_password: env_nonempty("BLUE_BUBBLES_PASSWORD"),
            voice_from_number: env_nonempty("TWILIO_VOICE_FROM_NUMBER")
                .or_else(|| env_nonempty("TWILIO_FROM_NUMBER")),
            postmark_url: POSTMARK_SERVER_URL.into(),
            slack_url: SLACK_AUTH_TEST_URL.into(),
            discord_url: DISCORD_AUTH_TEST_URL.into(),
            twilio_url,
            expo_url: EXPO_SERVER_INFO_URL.into(),
        }
    }
}

fn env_nonempty(name: &str) -> Option<String> {
    argo_config::cred::get(name).ok().filter(|v| !v.trim().is_empty())
}

pub(super) fn check(status: impl Into<String>, detail: impl Into<String>) -> Value {
    json!({"status": status.into(), "detail": detail.into()})
}

/// Probe configured approval channels without sending notifications.
pub(super) async fn collect() -> Value {
    collect_with(ProbeConfig::from_env()).await
}

pub(super) async fn collect_with(cfg: ProbeConfig) -> Value {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build();
    let Ok(client) = client else {
        return json!({
            "email": check("ERROR", "could not build HTTP client"),
            "slack": check("ERROR", "could not build HTTP client"),
            "discord": check("ERROR", "could not build HTTP client"),
            "sms": check("ERROR", "could not build HTTP client"),
            "push": check("ERROR", "could not build HTTP client"),
            "imessage": check("ERROR", "could not build HTTP client"),
            "voice": check("ERROR", "could not build HTTP client"),
        });
    };
    json!({
        "email": kinds::check_email(&client, &cfg).await,
        "slack": kinds::check_slack(&client, &cfg).await,
        "discord": kinds::check_discord(&client, &cfg).await,
        "sms": kinds::check_sms(&client, &cfg).await,
        "push": kinds::check_push(&client, &cfg).await,
        "imessage": kinds::check_imessage(&client, &cfg).await,
        "voice": kinds::check_voice(&client, &cfg).await,
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    pub(super) fn s(v: &Value) -> &str {
        v["status"].as_str().unwrap()
    }

    pub(super) async fn server(status: u16, headers: &[(&str, &str)], accepts: usize) -> String {
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

    pub(super) fn full_cfg(url: &str) -> ProbeConfig {
        ProbeConfig {
            postmark_token: Some("postmark-token".into()),
            slack_token: Some("slack-token".into()),
            discord_token: Some("discord-token".into()),
            twilio_sid: Some("AC_test".into()),
            twilio_token: Some("twilio-token".into()),
            expo_token: Some("expo-token".into()),
            bluebubbles_url: None,
            bluebubbles_password: None,
            voice_from_number: None,
            postmark_url: url.into(),
            slack_url: url.into(),
            discord_url: url.into(),
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
        let url = server(200, &[], 5).await;
        let v = collect_with(full_cfg(&url)).await;
        assert_eq!(s(&v["email"]), "OK");
        assert_eq!(s(&v["slack"]), "OK");
        assert_eq!(s(&v["discord"]), "OK");
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
