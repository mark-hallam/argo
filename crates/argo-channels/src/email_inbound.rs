//! Email inbound stub implementing [`crate::slack_inbound::InboundChannel`].
//!
//! The full IMAP/SES wiring is deferred to a follow-up brief — this
//! module exists to prove the [`InboundChannel`] trait is not
//! Slack-shaped. The signature envelope mirrors Postmark's webhook
//! shape (a single HMAC over the raw JSON body), and the approval
//! token is pulled from the URL clicked in the reply preview, which
//! Postmark echoes as `MailboxHash` or in the body itself.
//!
//! ```
//! use argo_channels::email_inbound::EmailInbound;
//! use argo_channels::slack_inbound::InboundChannel;
//! let inbound = EmailInbound::new("hmac-secret");
//! assert_eq!(inbound.id().to_string(), "email");
//! ```

use crate::slack_inbound::InboundChannel;
use crate::token::{verify_approval_token, ApprovalTokenClaims};
use crate::{ChannelError, ChannelId, ChannelResult};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Email inbound stub. Verifies a single Postmark-style HMAC over the
/// raw body and pulls the approval token from the JSON `Token` field
/// (the deferred IMAP/SES wiring will populate the same field).
pub struct EmailInbound {
    hmac_secret: String,
}

impl EmailInbound {
    /// Build the stub with the channel HMAC secret.
    #[must_use]
    pub fn new(hmac_secret: impl Into<String>) -> Self {
        Self {
            hmac_secret: hmac_secret.into(),
        }
    }
}

impl InboundChannel for EmailInbound {
    fn id(&self) -> ChannelId {
        ChannelId::new("email")
    }

    fn verify_signature(&self, body: &[u8], headers: &[(&str, &str)]) -> ChannelResult<()> {
        let sig = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Argo-Email-Signature"))
            .map(|(_, v)| *v)
            .ok_or_else(|| ChannelError::Auth("missing X-Argo-Email-Signature".into()))?;
        let mut mac = HmacSha256::new_from_slice(self.hmac_secret.as_bytes())
            .map_err(|e| ChannelError::Auth(format!("hmac init: {e}")))?;
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        let trimmed = sig.trim();
        if expected.len() != trimmed.len() {
            return Err(ChannelError::Auth("email signature length mismatch".into()));
        }
        let mut diff = 0_u8;
        for (a, b) in expected.as_bytes().iter().zip(trimmed.as_bytes().iter()) {
            diff |= a ^ b;
        }
        if diff != 0 {
            return Err(ChannelError::Auth("email signature mismatch".into()));
        }
        Ok(())
    }

    fn decode_approval_token(&self, body: &[u8]) -> ChannelResult<ApprovalTokenClaims> {
        let parsed: Value = serde_json::from_slice(body)
            .map_err(|e| ChannelError::Parse(format!("email body JSON: {e}")))?;
        let token = parsed
            .get("Token")
            .and_then(Value::as_str)
            .ok_or_else(|| ChannelError::Parse("email payload missing Token".into()))?;
        verify_approval_token(token, &self.hmac_secret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{sign_approval_token, ApprovalDecision};
    use chrono::Utc;

    #[test]
    fn email_trait_impl_round_trips_token() {
        let inbound = EmailInbound::new("email-hmac");
        let exp = Utc::now().timestamp() + 600;
        let token =
            sign_approval_token("act-99", ApprovalDecision::Reject, exp, "email-hmac").unwrap();
        let body = serde_json::json!({"Token": token, "From": "op@example.com"}).to_string();
        let mut mac = HmacSha256::new_from_slice(b"email-hmac").unwrap();
        mac.update(body.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let headers = [("X-Argo-Email-Signature", sig.as_str())];
        inbound
            .verify_signature(body.as_bytes(), &headers)
            .expect("signature verifies");
        let claims = inbound
            .decode_approval_token(body.as_bytes())
            .expect("token decodes");
        assert_eq!(claims.approval_id, "act-99");
        assert_eq!(claims.decision, ApprovalDecision::Reject);
    }
}
