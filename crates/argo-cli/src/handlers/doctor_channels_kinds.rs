//! Per-channel-kind diagnostic probes used by `argo doctor`.
//!
//! Each `check_*` function inspects one notification channel
//! (postmark email, slack, discord, twilio sms/voice, expo push,
//! blue_bubbles imessage) without sending a real notification, and
//! returns a `Value` describing its status.

use super::{check, ProbeConfig};
use serde_json::{json, Value};

const BLUE_BUBBLES_INFO_PATH: &str = "/api/v1/server/info";

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

pub(super) async fn check_email(client: &reqwest::Client, cfg: &ProbeConfig) -> Value {
    match cfg.postmark_token.as_deref() {
        Some(token) => {
            probe(
                client,
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
    }
}

pub(super) async fn check_slack(client: &reqwest::Client, cfg: &ProbeConfig) -> Value {
    match cfg.slack_token.as_deref() {
        Some(token) => {
            probe(
                client,
                ProbeSpec {
                    name: "slack",
                    url: cfg.slack_url.clone(),
                    auth: Auth::Bearer(token),
                },
            )
            .await
        }
        None => check("MISSING", "SLACK_APPROVALS_BOT_TOKEN is not set"),
    }
}

pub(super) async fn check_discord(client: &reqwest::Client, cfg: &ProbeConfig) -> Value {
    match cfg.discord_token.as_deref() {
        Some(token) => {
            probe(
                client,
                ProbeSpec {
                    name: "discord",
                    url: cfg.discord_url.clone(),
                    auth: Auth::Bearer(token),
                },
            )
            .await
        }
        None => check("MISSING", "DISCORD_BOT_TOKEN is not set"),
    }
}

pub(super) async fn check_sms(client: &reqwest::Client, cfg: &ProbeConfig) -> Value {
    twilio_probe(client, cfg, "twilio").await
}

pub(super) async fn check_push(client: &reqwest::Client, cfg: &ProbeConfig) -> Value {
    match cfg.expo_token.as_deref() {
        Some(token) => {
            probe(
                client,
                ProbeSpec {
                    name: "expo",
                    url: cfg.expo_url.clone(),
                    auth: Auth::Bearer(token),
                },
            )
            .await
        }
        None => check("MISSING", "EXPO_ACCESS_TOKEN is not set"),
    }
}

pub(super) async fn check_imessage(client: &reqwest::Client, cfg: &ProbeConfig) -> Value {
    let (url, password) = match (
        cfg.bluebubbles_url.as_deref(),
        cfg.bluebubbles_password.as_deref(),
    ) {
        (None, _) => return check("MISSING", "BLUE_BUBBLES_URL is not set"),
        (_, None) => return check("MISSING", "BLUE_BUBBLES_PASSWORD is not set"),
        (Some(url), Some(password)) => (url, password),
    };
    let info_url = format!("{}{}", url.trim_end_matches('/'), BLUE_BUBBLES_INFO_PATH);
    let resp = match client
        .get(&info_url)
        .header("Accept", "application/json")
        .query(&[("password", password)])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => return check("ERROR", format!("blue_bubbles network failure: {e}")),
    };
    let status = resp.status();
    let retry_after = resp
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    if status.as_u16() == 429 {
        let detail = retry_after.as_deref().map_or_else(
            || "blue_bubbles rate-limited".to_string(),
            |v| format!("blue_bubbles rate-limited; retry-after={v}"),
        );
        return check("WARN", detail)
            .with("http_status", status.as_u16())
            .with_opt("retry_after", retry_after);
    }
    if status.is_success() {
        return check("OK", format!("blue_bubbles reachable ({status})"))
            .with("http_status", status.as_u16());
    }
    check("ERROR", format!("blue_bubbles returned {status}"))
        .with("http_status", status.as_u16())
}

pub(super) async fn check_voice(client: &reqwest::Client, cfg: &ProbeConfig) -> Value {
    if cfg.voice_from_number.is_none() {
        return check(
            "MISSING",
            "TWILIO_VOICE_FROM_NUMBER (or TWILIO_FROM_NUMBER) is not set",
        );
    }
    twilio_probe(client, cfg, "twilio_voice").await
}

async fn twilio_probe(client: &reqwest::Client, cfg: &ProbeConfig, name: &'static str) -> Value {
    match (
        cfg.twilio_sid.as_deref(),
        cfg.twilio_token.as_deref(),
        cfg.twilio_url.as_ref(),
    ) {
        (Some(sid), Some(token), Some(url)) => {
            probe(
                client,
                ProbeSpec {
                    name,
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
    }
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
    use super::super::tests::{full_cfg, s, server};
    use super::super::{collect_with, ProbeConfig};

    #[tokio::test]
    async fn imessage_ok_when_configured() {
        let url = server(200, &[], 6).await;
        let cfg = ProbeConfig {
            bluebubbles_url: Some(url.clone()),
            bluebubbles_password: Some("bb-secret".into()),
            ..full_cfg(&url)
        };
        let v = collect_with(cfg).await;
        assert_eq!(s(&v["imessage"]), "OK");
    }

    #[tokio::test]
    async fn imessage_missing_when_url_unset() {
        let url = server(200, &[], 5).await;
        let cfg = full_cfg(&url);
        let v = collect_with(cfg).await;
        assert_eq!(s(&v["imessage"]), "MISSING");
        assert!(v["imessage"]["detail"]
            .as_str()
            .unwrap()
            .contains("BLUE_BUBBLES_URL"));
    }

    #[tokio::test]
    async fn voice_ok_when_configured() {
        let url = server(200, &[], 6).await;
        let cfg = ProbeConfig {
            voice_from_number: Some("+15555550111".into()),
            ..full_cfg(&url)
        };
        let v = collect_with(cfg).await;
        assert_eq!(s(&v["voice"]), "OK");
    }

    #[tokio::test]
    async fn voice_missing_when_from_number_unset() {
        let url = server(200, &[], 5).await;
        let cfg = full_cfg(&url);
        let v = collect_with(cfg).await;
        assert_eq!(s(&v["voice"]), "MISSING");
        assert!(v["voice"]["detail"]
            .as_str()
            .unwrap()
            .contains("TWILIO_VOICE_FROM_NUMBER"));
    }
}
