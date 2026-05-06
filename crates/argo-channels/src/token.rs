//! HMAC-signed approval tokens shared by outbound links and webhooks.

use crate::{ChannelError, ChannelResult};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Operator decision embedded in signed approval URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecision {
    /// Approve the pending action.
    Approve,
    /// Reject the pending action.
    Reject,
}

impl ApprovalDecision {
    /// String value used in signed token payloads.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
        }
    }

    /// Parse a decision string.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "approve" => Some(Self::Approve),
            "reject" => Some(Self::Reject),
            _ => None,
        }
    }
}

/// Verified approval token claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalTokenClaims {
    /// Approval id, currently the worker action id.
    pub approval_id: String,
    /// Decision carried by the URL that was clicked/replied with.
    pub decision: ApprovalDecision,
    /// Expiry as Unix seconds.
    pub exp: i64,
}

/// Sign approval claims into a compact URL token.
pub fn sign_approval_token(
    approval_id: &str,
    decision: ApprovalDecision,
    exp: i64,
    secret: &str,
) -> ChannelResult<String> {
    if secret.is_empty() {
        return Err(ChannelError::Config(
            "ARGO_CHANNEL_HMAC_SECRET must not be empty".into(),
        ));
    }
    let payload = token_payload(approval_id, decision, exp);
    let sig = hmac_hex(payload.as_bytes(), secret.as_bytes())?;
    Ok(format!(
        "{}.{}.{}.{}",
        approval_id,
        decision.as_str(),
        exp,
        sig
    ))
}

/// Verify a compact approval token.
pub fn verify_approval_token(token: &str, secret: &str) -> ChannelResult<ApprovalTokenClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 4 {
        return Err(ChannelError::Auth(
            "approval token must have 4 fields".into(),
        ));
    }
    let decision = ApprovalDecision::parse(parts[1])
        .ok_or_else(|| ChannelError::Auth("invalid approval decision".into()))?;
    let exp = parts[2]
        .parse::<i64>()
        .map_err(|e| ChannelError::Auth(format!("invalid approval token exp: {e}")))?;
    let payload = token_payload(parts[0], decision, exp);
    let expected = hmac_hex(payload.as_bytes(), secret.as_bytes())?;
    if !constant_time_eq(expected.as_bytes(), parts[3].as_bytes()) {
        return Err(ChannelError::Auth(
            "approval token signature mismatch".into(),
        ));
    }
    if Utc::now().timestamp() > exp {
        return Err(ChannelError::Auth("approval token expired".into()));
    }
    Ok(ApprovalTokenClaims {
        approval_id: parts[0].to_string(),
        decision,
        exp,
    })
}

/// Build a decision URL with a signed `token` query parameter.
pub fn signed_decision_url(
    base_url: &str,
    approval_id: &str,
    decision: ApprovalDecision,
    exp: i64,
    secret: &str,
) -> ChannelResult<String> {
    let token = sign_approval_token(approval_id, decision, exp, secret)?;
    let sep = if base_url.contains('?') { '&' } else { '?' };
    Ok(format!("{base_url}{sep}token={token}"))
}

/// Verify a Postmark-style webhook signature over the raw JSON body.
pub fn verify_webhook_signature(
    body: &[u8],
    signature_hex: &str,
    secret: &str,
) -> ChannelResult<()> {
    let expected = hmac_hex(body, secret.as_bytes())?;
    if constant_time_eq(expected.as_bytes(), signature_hex.trim().as_bytes()) {
        Ok(())
    } else {
        Err(ChannelError::Auth("webhook signature mismatch".into()))
    }
}

fn token_payload(approval_id: &str, decision: ApprovalDecision, exp: i64) -> String {
    format!("{}|{}|{}", approval_id, decision.as_str(), exp)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trips_and_rejects_tamper() {
        let exp = Utc::now().timestamp() + 60;
        let token =
            sign_approval_token("act-1", ApprovalDecision::Approve, exp, "secret").expect("token");
        let claims = verify_approval_token(&token, "secret").expect("verify");
        assert_eq!(claims.approval_id, "act-1");
        assert_eq!(claims.decision, ApprovalDecision::Approve);
        let tampered = token.replace("approve", "reject");
        assert!(verify_approval_token(&tampered, "secret").is_err());
    }

    #[test]
    fn webhook_signature_checks_body_hmac() {
        let sig = hmac_hex(br#"{"ok":true}"#, b"secret").expect("sig");
        verify_webhook_signature(br#"{"ok":true}"#, &sig, "secret").expect("valid");
        assert!(verify_webhook_signature(br#"{"ok":false}"#, &sig, "secret").is_err());
    }
}
