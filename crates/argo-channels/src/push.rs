//! Expo-backed push approval notification channel.

use crate::rate_limit::{RateLimit, RateLimitConfig};
use crate::{
    push_format::{action_summary, expo_has_error, expo_message_id, token_from_url},
    ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
    Notification, NotifyChannel,
};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

pub use crate::push_format::{
    device_registry_path_from_env, is_gateway_route, load_devices_for_operator,
    parse_gateway_request, register_device, verify_decision_token, PushDevice, PushDeviceInput,
    PushGatewayRequest,
};

const DEFAULT_EXPO_ENDPOINT: &str = "https://exp.host/--/api/v2/push/send";
#[cfg(test)] const EXPO_TEST_TOKEN_ENV: &str = "EXPO_TEST_TOKEN";

/// Configuration for [`PushApprovalChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushApprovalChannelConfig {
    channel_id: ChannelId,
    endpoint: String,
    access_token: Option<String>,
    device_registry_path: PathBuf,
    /// Operator override for the per-channel rate-limit budget.
    /// Missing fields fall back to the push default
    /// (5/sec, 100/min).
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
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
            rate_limit: RateLimitConfig::default(),
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
            rate_limit: RateLimitConfig::unlimited(),
        }
    }
}

/// Expo push approval channel.
pub struct PushApprovalChannel {
    config: PushApprovalChannelConfig,
    client: reqwest::Client,
    rate_limit: RateLimit,
}

impl PushApprovalChannel {
    /// Create a push channel.
    #[must_use]
    pub fn new(config: PushApprovalChannelConfig) -> Self {
        let rate_limit = RateLimit::from_config(
            RateLimitConfig::push_default().with_override(config.rate_limit),
        );
        Self {
            config,
            client: reqwest::Client::new(),
            rate_limit,
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
        if let Err(retry_after) = self.rate_limit.try_acquire() {
            return Err(ChannelError::RateLimited { retry_after });
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{self, ApprovalDecision};
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
