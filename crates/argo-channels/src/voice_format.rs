//! Voice channel DTMF token signing and TwiML rendering helpers.
//!
//! Split out of `voice.rs` to keep the channel send loop and provider
//! transport in one file and the protocol/format helpers in another.

use crate::{ChannelError, ChannelResult};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// Operator decision captured from Twilio DTMF digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VoiceDecision {
    /// Operator pressed 1 to approve.
    Approve,
    /// Operator pressed 2 to deny.
    Deny,
    /// Operator pressed 9 to request more details.
    RequestDetails,
}

impl VoiceDecision {
    /// Parse the single DTMF digit used by the voice approval flow.
    #[must_use]
    pub fn from_digits(digits: &str) -> Option<Self> {
        match digits.trim() {
            "1" => Some(Self::Approve),
            "2" => Some(Self::Deny),
            "9" => Some(Self::RequestDetails),
            _ => None,
        }
    }

    /// Return the stable decision string used in audit payloads.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Deny => "deny",
            Self::RequestDetails => "request-details",
        }
    }
}

/// Verified HMAC token claims from a Twilio DTMF callback URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceCallbackClaims {
    /// Approval id the callback may settle.
    pub approval_id: String,
    /// Worker action id included for audit/debug correlation.
    pub action_id: String,
    /// Operator id targeted by the outbound call.
    pub operator_id: String,
    /// Unix expiry timestamp.
    pub exp: i64,
    /// Per-call nonce to make callback URLs unique.
    pub nonce: String,
}

/// Sign a DTMF callback token for the voice channel.
pub fn sign_voice_callback_token(
    approval_id: &str,
    action_id: &str,
    operator_id: &str,
    exp: i64,
    nonce: &str,
    secret: &str,
) -> ChannelResult<String> {
    if secret.is_empty() {
        return Err(ChannelError::Config(
            "voice callback secret must not be empty".into(),
        ));
    }
    for (name, value) in [
        ("approval_id", approval_id),
        ("action_id", action_id),
        ("operator_id", operator_id),
        ("nonce", nonce),
    ] {
        if value.is_empty() || value.contains('|') {
            return Err(ChannelError::Config(format!(
                "voice callback {name} must be non-empty and not contain |"
            )));
        }
    }
    let payload = format!("{approval_id}|{action_id}|{operator_id}|{exp}|{nonce}");
    Ok(format!(
        "{}.{}",
        hex::encode(payload.as_bytes()),
        hmac_sha256_hex(payload.as_bytes(), secret.as_bytes())?
    ))
}

/// Verify a signed DTMF callback token and reject expired callbacks.
pub fn verify_voice_callback_token(
    token: &str,
    secret: &str,
) -> ChannelResult<VoiceCallbackClaims> {
    if secret.is_empty() {
        return Err(ChannelError::Auth(
            "voice callback secret must not be empty".into(),
        ));
    }
    let (payload_hex, sig) = token.rsplit_once('.').ok_or_else(|| {
        ChannelError::Auth("voice callback token must have payload.signature".into())
    })?;
    let payload_bytes = hex::decode(payload_hex)
        .map_err(|e| ChannelError::Auth(format!("voice callback token payload hex: {e}")))?;
    let payload = std::str::from_utf8(&payload_bytes)
        .map_err(|e| ChannelError::Auth(format!("voice callback token payload utf8: {e}")))?;
    let expected = hmac_sha256_hex(payload.as_bytes(), secret.as_bytes())?;
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
        return Err(ChannelError::Auth(
            "voice callback token signature mismatch".into(),
        ));
    }
    let parts: Vec<&str> = payload.split('|').collect();
    if parts.len() != 5 {
        return Err(ChannelError::Auth(
            "voice callback token payload must have 5 fields".into(),
        ));
    }
    let exp = parts[3]
        .parse::<i64>()
        .map_err(|e| ChannelError::Auth(format!("voice callback token exp: {e}")))?;
    if Utc::now().timestamp() > exp {
        return Err(ChannelError::Auth("voice callback token expired".into()));
    }
    Ok(VoiceCallbackClaims {
        approval_id: parts[0].to_string(),
        action_id: parts[1].to_string(),
        operator_id: parts[2].to_string(),
        exp,
        nonce: parts[4].to_string(),
    })
}

/// Render TwiML that speaks the action description and gathers one DTMF key.
#[must_use]
pub fn render_voice_twiml(action_description: &str, callback_url: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<Response><Gather input=\"dtmf\" numDigits=\"1\" timeout=\"10\" action=\"{}\" method=\"POST\">\
<Say voice=\"alice\">{} Press 1 to approve. Press 2 to deny. Press 9 for more details.</Say>\
</Gather><Say>No input received. Goodbye.</Say></Response>",
        xml_escape(callback_url),
        xml_escape(action_description)
    )
}

fn hmac_sha256_hex(data: &[u8], secret: &[u8]) -> ChannelResult<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|e| ChannelError::Auth(format!("voice callback hmac init: {e}")))?;
    mac.update(data);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0_u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dtmf_digits_map_to_decisions() {
        assert_eq!(
            VoiceDecision::from_digits("1"),
            Some(VoiceDecision::Approve)
        );
        assert_eq!(VoiceDecision::from_digits("2"), Some(VoiceDecision::Deny));
        assert_eq!(
            VoiceDecision::from_digits("9"),
            Some(VoiceDecision::RequestDetails)
        );
        assert_eq!(VoiceDecision::from_digits("3"), None);
    }

    #[test]
    fn twiml_say_template_renders_action_description() {
        let twiml = render_voice_twiml("Approve <refund>?", "https://gw/twilio?vt=abc&x=1");
        assert!(twiml.contains("<Gather"));
        assert!(twiml.contains("<Say voice=\"alice\">Approve &lt;refund&gt;?"));
        assert!(twiml.contains("action=\"https://gw/twilio?vt=abc&amp;x=1\""));
    }

    #[test]
    fn voice_callback_token_verifies_claims() {
        let exp = Utc::now().timestamp() + 60;
        let token = sign_voice_callback_token("appr", "act", "op", exp, "nonce", "secret").unwrap();
        let claims = verify_voice_callback_token(&token, "secret").unwrap();
        assert_eq!(claims.approval_id, "appr");
        assert_eq!(claims.action_id, "act");
        assert_eq!(claims.operator_id, "op");
    }

    #[test]
    fn expired_voice_callback_token_is_rejected() {
        let exp = Utc::now().timestamp() - 1;
        let token = sign_voice_callback_token("appr", "act", "op", exp, "nonce", "secret").unwrap();
        assert!(verify_voice_callback_token(&token, "secret").is_err());
    }
}
