//! Slack inbound webhook signature verification + approval-token decode.
//!
//! Companion to [`crate::slack`] (outbound `chat.postMessage`). Where the
//! outbound side renders Block Kit buttons carrying signed approval
//! tokens, this module verifies the signature Slack stamps on the
//! interactive callback and pulls the approval-token claims back out
//! of the `payload` form field. The HTTP wiring lives in the gateway
//! crate; everything testable lives here so the crate that owns
//! channel transports also owns inbound proof.
//!
//! ```
//! use argo_channels::slack_inbound::{InboundChannel, SlackInbound, SlackInboundConfig};
//! let inbound = SlackInbound::new(SlackInboundConfig {
//!     signing_secret: "test-secret".into(),
//!     hmac_secret: "approval-secret".into(),
//!     max_skew_seconds: 300,
//! });
//! let _id = inbound.id();
//! ```

use crate::token::{verify_approval_token, ApprovalTokenClaims};
use crate::{ChannelError, ChannelId, ChannelResult};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Minimum trait that every inbound webhook channel implements.
///
/// Intentionally narrow: the only two pieces of channel-specific
/// logic that callers cannot share are signature verification (the
/// HMAC envelope every provider uses to prove a request came from
/// them) and approval-token decoding (some providers stash the token
/// in a JSON field, others in a form value, others in a URL).
///
/// The route handler in the gateway crate composes both: verify
/// signature → decode token → call the existing state-machine
/// `approve`/`reject` transition.
///
/// ```
/// use argo_channels::slack_inbound::{InboundChannel, SlackInbound, SlackInboundConfig};
/// let inbound = SlackInbound::new(SlackInboundConfig {
///     signing_secret: "shh".into(),
///     hmac_secret: "tok".into(),
///     max_skew_seconds: 300,
/// });
/// assert_eq!(inbound.id().to_string(), "slack");
/// ```
pub trait InboundChannel: Send + Sync {
    /// Stable channel id (matches the outbound channel id).
    fn id(&self) -> ChannelId;

    /// Verify the provider's signature over the raw request body.
    ///
    /// `headers` is provided as `(name, value)` pairs in arrival
    /// order. Missing required headers MUST produce
    /// [`ChannelError::Auth`].
    fn verify_signature(&self, body: &[u8], headers: &[(&str, &str)]) -> ChannelResult<()>;

    /// Pull the approval-token claims from the parsed payload.
    ///
    /// Implementations are responsible for shape-specific extraction
    /// (Slack: `actions[0].value`, Email: signed URL query string).
    /// The HMAC verification of the token itself is delegated to
    /// [`crate::token::verify_approval_token`].
    fn decode_approval_token(&self, body: &[u8]) -> ChannelResult<ApprovalTokenClaims>;
}

/// Static configuration for [`SlackInbound`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackInboundConfig {
    /// Slack signing secret from `SLACK_APPROVALS_SIGNING_SECRET`.
    /// Used to compute the `v0=…` HMAC over `v0:<ts>:<body>`.
    pub signing_secret: String,
    /// Argo channel HMAC secret from `ARGO_CHANNEL_HMAC_SECRET`. Used
    /// to verify the signed approval token Slack echoes back in the
    /// button's `value` field.
    pub hmac_secret: String,
    /// Maximum acceptable skew in seconds between the
    /// `X-Slack-Request-Timestamp` header and wall clock. Slack
    /// recommends 5 minutes (300 s) for replay protection.
    pub max_skew_seconds: i64,
}

impl SlackInboundConfig {
    /// Build a config with the Slack-recommended 5-minute skew window.
    #[must_use]
    pub fn new(signing_secret: impl Into<String>, hmac_secret: impl Into<String>) -> Self {
        Self {
            signing_secret: signing_secret.into(),
            hmac_secret: hmac_secret.into(),
            max_skew_seconds: 300,
        }
    }
}

/// Slack inbound webhook implementation of [`InboundChannel`].
///
/// ```
/// use argo_channels::slack_inbound::{InboundChannel, SlackInbound, SlackInboundConfig};
/// let inbound = SlackInbound::new(SlackInboundConfig::new("sigsecret", "hmac"));
/// assert_eq!(inbound.id().to_string(), "slack");
/// ```
pub struct SlackInbound {
    config: SlackInboundConfig,
}

impl SlackInbound {
    /// Build a new Slack inbound verifier from config.
    #[must_use]
    pub fn new(config: SlackInboundConfig) -> Self {
        Self { config }
    }

    /// Borrow the active config (test/diagnostic helper).
    #[must_use]
    pub fn config(&self) -> &SlackInboundConfig {
        &self.config
    }
}

impl InboundChannel for SlackInbound {
    fn id(&self) -> ChannelId {
        ChannelId::new("slack")
    }

    fn verify_signature(&self, body: &[u8], headers: &[(&str, &str)]) -> ChannelResult<()> {
        let signature = lookup_header(headers, "X-Slack-Signature")
            .ok_or_else(|| ChannelError::Auth("missing X-Slack-Signature header".into()))?;
        let timestamp = lookup_header(headers, "X-Slack-Request-Timestamp")
            .ok_or_else(|| ChannelError::Auth("missing X-Slack-Request-Timestamp header".into()))?;
        let ts_num = timestamp
            .parse::<i64>()
            .map_err(|e| ChannelError::Auth(format!("invalid Slack timestamp: {e}")))?;
        let now = Utc::now().timestamp();
        if (now - ts_num).abs() > self.config.max_skew_seconds {
            return Err(ChannelError::Auth(format!(
                "Slack request timestamp skew exceeds {}s",
                self.config.max_skew_seconds
            )));
        }
        let body_str = std::str::from_utf8(body)
            .map_err(|e| ChannelError::Parse(format!("Slack body utf8: {e}")))?;
        let basestring = format!("v0:{timestamp}:{body_str}");
        let computed = hmac_hex(basestring.as_bytes(), self.config.signing_secret.as_bytes())?;
        let expected = format!("v0={computed}");
        if !constant_time_eq(expected.as_bytes(), signature.trim().as_bytes()) {
            return Err(ChannelError::Auth("Slack signature mismatch".into()));
        }
        Ok(())
    }

    fn decode_approval_token(&self, body: &[u8]) -> ChannelResult<ApprovalTokenClaims> {
        let body_str = std::str::from_utf8(body)
            .map_err(|e| ChannelError::Parse(format!("Slack body utf8: {e}")))?;
        let payload_json = form_value(body_str, "payload").unwrap_or_else(|| body_str.to_string());
        let parsed: Value = serde_json::from_str(&payload_json)
            .map_err(|e| ChannelError::Parse(format!("Slack payload JSON: {e}")))?;
        let token = parsed
            .get("actions")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|act| act.get("value"))
            .and_then(Value::as_str)
            .ok_or_else(|| ChannelError::Parse("Slack payload missing actions[0].value".into()))?;
        verify_approval_token(token, &self.config.hmac_secret)
    }
}

fn lookup_header<'a>(headers: &'a [(&str, &str)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| *v)
}

fn hmac_hex(data: &[u8], secret: &[u8]) -> ChannelResult<String> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| ChannelError::Auth(format!("hmac init: {e}")))?;
    mac.update(data);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn form_value(body: &str, key: &str) -> Option<String> {
    body.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        if percent_decode(k).as_deref() == Some(key) {
            percent_decode(v)
        } else {
            None
        }
    })
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

/// Compute the Slack `v0=…` signature for a body+timestamp under a
/// given signing secret. Exposed for fixture authoring and tests in
/// other crates that need to construct a valid signed request.
///
/// ```
/// use argo_channels::slack_inbound::sign_slack_v0;
/// let sig = sign_slack_v0(b"payload=%7B%7D", "1700000000", "secret").unwrap();
/// assert!(sig.starts_with("v0="));
/// ```
pub fn sign_slack_v0(body: &[u8], timestamp: &str, signing_secret: &str) -> ChannelResult<String> {
    let body_str = std::str::from_utf8(body)
        .map_err(|e| ChannelError::Parse(format!("body utf8: {e}")))?;
    let basestring = format!("v0:{timestamp}:{body_str}");
    let mac = hmac_hex(basestring.as_bytes(), signing_secret.as_bytes())?;
    Ok(format!("v0={mac}"))
}

#[cfg(test)]
#[path = "slack_inbound_tests.rs"]
mod tests;
