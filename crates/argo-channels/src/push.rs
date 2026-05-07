//! Expo-backed push approval notification channel.

use crate::{
    token::{self, ApprovalDecision},
    ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
    Notification, NotifyChannel,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const DEFAULT_EXPO_ENDPOINT: &str = "https://exp.host/--/api/v2/push/send";
#[cfg(test)] const EXPO_TEST_TOKEN_ENV: &str = "EXPO_TEST_TOKEN";

/// Registered Expo device token for one operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PushDevice {
    operator_id: String,
    expo_push_token: String,
    platform: String,
    registered_at: DateTime<Utc>,
}

impl PushDevice {
    /// Build a new registration stamped with the current time.
    #[must_use]
    pub fn new(operator_id: String, expo_push_token: String, platform: String) -> Self {
        Self { operator_id, expo_push_token, platform, registered_at: Utc::now() }
    }
}

/// Device registration body accepted by the gateway.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PushDeviceInput {
    operator_id: String,
    expo_push_token: String,
    platform: String,
}

impl PushDeviceInput {
    /// Convert the request body into a stored device row.
    #[must_use]
    pub fn into_device(self) -> PushDevice {
        PushDevice::new(self.operator_id, self.expo_push_token, self.platform)
    }
}

/// Parsed push gateway request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushGatewayRequest {
    /// Register or update one operator device.
    Register(PushDeviceInput),
    /// Settle an approval from an in-app deep link.
    Decide {
        /// Signed approval token from the push deep link.
        token: String,
        /// Requested decision from query parameter `d`.
        decision: Option<ApprovalDecision>,
    },
}

/// Configuration for [`PushApprovalChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushApprovalChannelConfig {
    channel_id: ChannelId,
    endpoint: String,
    access_token: Option<String>,
    device_registry_path: PathBuf,
}

impl PushApprovalChannelConfig {
    /// Build config from `EXPO_ACCESS_TOKEN`, `EXPO_PUSH_ENDPOINT`, and
    /// `ARGO_PUSH_DEVICES_PATH` when present.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            channel_id: ChannelId::new("push"),
            endpoint: std::env::var("EXPO_PUSH_ENDPOINT")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_EXPO_ENDPOINT.into()),
            access_token: argo_config::cred::get("EXPO_ACCESS_TOKEN")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            device_registry_path: device_registry_path_from_env(),
        }
    }

    /// Local config pointing at a supplied device registry.
    #[must_use]
    pub fn local(device_registry_path: impl Into<PathBuf>) -> Self {
        Self {
            channel_id: ChannelId::new("push"),
            endpoint: DEFAULT_EXPO_ENDPOINT.into(),
            access_token: None,
            device_registry_path: device_registry_path.into(),
        }
    }
}

/// Expo push approval channel.
pub struct PushApprovalChannel {
    config: PushApprovalChannelConfig,
    client: reqwest::Client,
}

impl PushApprovalChannel {
    /// Create a push channel.
    #[must_use]
    pub fn new(config: PushApprovalChannelConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Build Expo push messages for all target devices.
    pub fn message_payloads(
        &self,
        n: &Notification,
        devices: &[PushDevice],
    ) -> ChannelResult<Vec<Value>> {
        let token = token_from_url(&n.approve_url)?;
        let deep_link = format!("argo://inbox/approval/{token}");
        let body = action_summary(n);
        Ok(devices
            .iter()
            .map(|device| {
                json!({
                    "to": device.expo_push_token,
                    "title": format!("Approval needed: {}", n.action_id),
                    "body": body,
                    "sound": "default",
                    "data": {
                        "url": deep_link,
                        "approval_id": n.approval_id,
                        "action_id": n.action_id,
                        "risk_level": n.risk_level.to_string(),
                        "decision_url": format!("/channels/push/decision/{token}")
                    }
                })
            })
            .collect())
    }

    fn devices_for(&self, operator_id: &str) -> ChannelResult<Vec<PushDevice>> {
        let devices = load_devices_for_operator(&self.config.device_registry_path, operator_id)?;
        if devices.is_empty() {
            Err(ChannelError::Config(format!(
                "no Expo push devices for operator {operator_id}"
            )))
        } else {
            Ok(devices)
        }
    }
}

#[async_trait]
impl NotifyChannel for PushApprovalChannel {
    fn id(&self) -> ChannelId {
        self.config.channel_id.clone()
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        let devices = self.devices_for(&notification.target.operator_id)?;
        let payloads = self.message_payloads(&notification, &devices)?;
        let mut req = self.client.post(&self.config.endpoint).json(&payloads);
        if let Some(access_token) = &self.config.access_token {
            req = req.bearer_auth(access_token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ChannelError::Provider(format!("expo push send: {e}")))?;
        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| ChannelError::Provider(format!("expo response: {e}")))?;
        if !status.is_success() {
            return Err(ChannelError::Provider(format!(
                "expo returned {status}: {raw}"
            )));
        }
        let parsed: Value = serde_json::from_str(&raw)
            .map_err(|e| ChannelError::Provider(format!("expo response JSON: {e}")))?;
        let provider_message_id = expo_message_id(&parsed);
        if expo_has_error(&parsed) {
            return Err(ChannelError::Provider(format!("expo push error: {raw}")));
        }
        Ok(DeliveryReceipt {
            channel_id: self.id(),
            notification_id: notification.approval_id,
            provider_message_id,
            status: DeliveryStatus::Sent,
            deferred_reason: None,
            delivered_at: Utc::now(),
        })
    }

    async fn poll(&self) -> ChannelResult<Vec<InboundEvent>> {
        Ok(Vec::new())
    }
}

/// Return true when an HTTP request targets the push gateway routes.
#[must_use]
pub fn is_gateway_route(req: &str) -> bool {
    req.starts_with("POST /channels/push/devices ")
        || req.starts_with("POST /channels/push/devices?")
        || req.starts_with("POST /channels/push/decision/")
}

/// Parse a raw gateway request into a push operation.
pub fn parse_gateway_request(req: &str) -> ChannelResult<PushGatewayRequest> {
    if req.starts_with("POST /channels/push/devices ")
        || req.starts_with("POST /channels/push/devices?")
    {
        let body = body_from_request(req);
        let input: PushDeviceInput = serde_json::from_str(body)
            .map_err(|e| ChannelError::Parse(format!("parse push device JSON: {e}")))?;
        validate_device_input(&input)?;
        return Ok(PushGatewayRequest::Register(input));
    }
    let Some(rest) = req.strip_prefix("POST /channels/push/decision/") else {
        return Err(ChannelError::Parse("unsupported push route".into()));
    };
    let target = rest.split_once(' ').map_or(rest, |(path, _)| path);
    let (token, query) = target.split_once('?').map_or((target, ""), |(t, q)| (t, q));
    let token = percent_decode(token)
        .ok_or_else(|| ChannelError::Parse("invalid push token encoding".into()))?;
    Ok(PushGatewayRequest::Decide {
        token,
        decision: query_decision(query)?,
    })
}

/// Verify a signed push decision token, optionally overriding its embedded
/// decision with query parameter `d`.
pub fn verify_decision_token(
    token: &str,
    requested: Option<ApprovalDecision>,
    secret: &str,
) -> ChannelResult<(String, ApprovalDecision)> {
    let claims = token::verify_approval_token(token, secret)?;
    Ok((claims.approval_id, requested.unwrap_or(claims.decision)))
}

/// Registry path from `ARGO_PUSH_DEVICES_PATH`, defaulting to `.skycot`.
#[must_use]
pub fn device_registry_path_from_env() -> PathBuf {
    std::env::var_os("ARGO_PUSH_DEVICES_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".skycot/push_devices.jsonl")
        })
}

/// Register or update a device row.
pub fn register_device(path: &Path, device: PushDevice) -> ChannelResult<()> {
    validate_device(&device)?;
    let mut devices = load_devices(path)?;
    devices.retain(|d| {
        !(d.operator_id == device.operator_id && d.expo_push_token == device.expo_push_token)
    });
    devices.push(device);
    devices.sort_by(|a, b| {
        (&a.operator_id, &a.platform, &a.expo_push_token).cmp(&(
            &b.operator_id,
            &b.platform,
            &b.expo_push_token,
        ))
    });
    save_devices(path, &devices)
}

/// Load registered devices for one operator.
pub fn load_devices_for_operator(path: &Path, operator_id: &str) -> ChannelResult<Vec<PushDevice>> {
    Ok(load_devices(path)?
        .into_iter()
        .filter(|d| d.operator_id == operator_id)
        .collect())
}

fn load_devices(path: &Path) -> ChannelResult<Vec<PushDevice>> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(ChannelError::Provider(format!("read push devices: {e}"))),
    };
    serde_json::from_str(&raw).or_else(|_| {
        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect()
    }).map_err(|e| ChannelError::Parse(format!("parse push devices: {e}")))
}

fn save_devices(path: &Path, devices: &[PushDevice]) -> ChannelResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ChannelError::Provider(format!("create push device dir: {e}")))?;
    }
    let raw = serde_json::to_string_pretty(devices)
        .map_err(|e| ChannelError::Provider(format!("serialize push devices: {e}")))?;
    std::fs::write(path, raw)
        .map_err(|e| ChannelError::Provider(format!("write push devices: {e}")))
}

fn validate_device_input(input: &PushDeviceInput) -> ChannelResult<()> {
    validate_nonempty("operator_id", &input.operator_id)?;
    validate_nonempty("expo_push_token", &input.expo_push_token)?;
    validate_nonempty("platform", &input.platform)
}

fn validate_device(device: &PushDevice) -> ChannelResult<()> {
    validate_nonempty("operator_id", &device.operator_id)?;
    validate_nonempty("expo_push_token", &device.expo_push_token)?;
    validate_nonempty("platform", &device.platform)
}

fn validate_nonempty(field: &str, value: &str) -> ChannelResult<()> {
    if value.trim().is_empty() {
        Err(ChannelError::Config(format!(
            "push device {field} is empty"
        )))
    } else {
        Ok(())
    }
}

fn body_from_request(req: &str) -> &str {
    req.split_once("\r\n\r\n").map_or("", |(_, body)| body)
}

fn query_decision(query: &str) -> ChannelResult<Option<ApprovalDecision>> {
    query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find_map(|(key, value)| {
            (percent_decode(key).as_deref() == Some("d"))
                .then(|| percent_decode(value))
                .flatten()
        })
        .map(|value| {
            ApprovalDecision::parse(&value)
                .ok_or_else(|| ChannelError::Parse("invalid push decision".into()))
        })
        .transpose()
}

fn token_from_url(url: &str) -> ChannelResult<String> {
    url.split_once('?')
        .and_then(|(_, q)| {
            q.split('&')
                .find_map(|kv| kv.strip_prefix("token=").and_then(percent_decode))
        })
        .ok_or_else(|| ChannelError::Config("push approval URL missing token".into()))
}

fn action_summary(n: &Notification) -> String {
    let first = n.prompt_body.lines().next().unwrap_or("").trim();
    let summary = if first.is_empty() {
        format!("{} risk approval requested", n.risk_level)
    } else {
        first.to_string()
    };
    truncate_chars(&summary, 180)
}

fn truncate_chars(s: &str, max: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if idx == max {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn expo_message_id(v: &Value) -> Option<String> {
    let data = v.get("data")?;
    if let Some(items) = data.as_array() {
        return items.first()?.get("id")?.as_str().map(str::to_string);
    }
    data.get("id")?.as_str().map(str::to_string)
}

fn expo_has_error(v: &Value) -> bool {
    match v.get("data") {
        Some(Value::Array(items)) => items
            .iter()
            .any(|item| item.get("status").and_then(Value::as_str) == Some("error")),
        Some(Value::Object(item)) => item.get("status").and_then(Value::as_str) == Some("error"),
        _ => false,
    }
}

fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                out.push((hex_val(bytes[i + 1])? << 4) | hex_val(bytes[i + 2])?);
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).ok()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OperatorTarget, RiskLevel};
    use chrono::Duration;

    fn notification() -> Notification {
        let exp = Utc::now().timestamp() + 60;
        let token =
            token::sign_approval_token("act-1", ApprovalDecision::Approve, exp, "secret").unwrap();
        Notification {
            target: OperatorTarget { operator_id: "op-1".into(), email: "op@example.com".into() },
            action_id: "act-1".into(),
            approval_id: "act-1".into(),
            risk_level: RiskLevel::High,
            prompt_body: "Approve native push action?".into(),
            approve_url: format!("https://gw/decision?token={token}"),
            reject_url: "https://gw/decision?token=reject".into(),
            expiry: Utc::now() + Duration::minutes(5),
        }
    }

    #[test]
    fn register_device_upserts_per_operator_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("devices.json");
        let device = PushDevice::new("op-1".into(), "ExponentPushToken[test]".into(), "ios".into());
        register_device(&path, device.clone()).unwrap();
        register_device(&path, device).unwrap();
        assert_eq!(load_devices_for_operator(&path, "op-1").unwrap().len(), 1);
    }

    #[test]
    fn payload_deep_links_to_mobile_inbox() {
        let channel = PushApprovalChannel::new(PushApprovalChannelConfig::local("devices.json"));
        let device = PushDevice::new("op-1".into(), "ExponentPushToken[test]".into(), "ios".into());
        let payloads = channel.message_payloads(&notification(), &[device]).unwrap();
        let url = payloads[0]["data"]["url"].as_str().unwrap();
        assert!(url.starts_with("argo://inbox/approval/act-1.approve."));
    }

    #[test]
    fn parses_gateway_registration_and_decision() {
        let req = "POST /channels/push/devices HTTP/1.1\r\n\r\n{\"operator_id\":\"op-1\",\"expo_push_token\":\"ExponentPushToken[test]\",\"platform\":\"ios\"}";
        assert!(matches!(parse_gateway_request(req).unwrap(), PushGatewayRequest::Register(_)));
        let req = "POST /channels/push/decision/abc.approve.1.sig?d=reject HTTP/1.1\r\n\r\n";
        assert!(matches!(
            parse_gateway_request(req).unwrap(),
            PushGatewayRequest::Decide {
                decision: Some(ApprovalDecision::Reject),
                ..
            }
        ));
    }

    #[tokio::test]
    async fn expo_integration_gated_on_env() {
        let Some(token) = std::env::var(EXPO_TEST_TOKEN_ENV).ok().filter(|v| !v.is_empty()) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("devices.json");
        register_device(&path, PushDevice::new("op-1".into(), token, "ios".into())).unwrap();
        let channel = PushApprovalChannel::new(PushApprovalChannelConfig::local(path));
        let receipt = channel.send(notification()).await.unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Sent);
    }
}
