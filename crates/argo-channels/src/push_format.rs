//! Push gateway parsing, device registry, and Expo payload helpers.
//!
//! Split out of `push.rs` to keep the channel send loop and provider
//! transport in one file and the protocol/storage helpers in another.

use crate::{
    token::{self, ApprovalDecision},
    ChannelError, ChannelResult, Notification,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Registered Expo device token for one operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PushDevice {
    pub(crate) operator_id: String,
    pub(crate) expo_push_token: String,
    pub(crate) platform: String,
    pub(crate) registered_at: DateTime<Utc>,
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

pub(crate) fn token_from_url(url: &str) -> ChannelResult<String> {
    url.split_once('?')
        .and_then(|(_, q)| {
            q.split('&')
                .find_map(|kv| kv.strip_prefix("token=").and_then(percent_decode))
        })
        .ok_or_else(|| ChannelError::Config("push approval URL missing token".into()))
}

pub(crate) fn action_summary(n: &Notification) -> String {
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

pub(crate) fn expo_message_id(v: &Value) -> Option<String> {
    let data = v.get("data")?;
    if let Some(items) = data.as_array() {
        return items.first()?.get("id")?.as_str().map(str::to_string);
    }
    data.get("id")?.as_str().map(str::to_string)
}

pub(crate) fn expo_has_error(v: &Value) -> bool {
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
