//! HMAC-signed approval tokens shared by outbound links and webhooks.

use crate::{ChannelError, ChannelResult};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub use argo_types::ApprovalDecision;

type HmacSha256 = Hmac<Sha256>;

/// Verified approval token claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalTokenClaims {
    /// Approval id, currently the worker action id.
    pub approval_id: String,
    /// Decision carried by the URL that was clicked/replied with.
    pub decision: ApprovalDecision,
    /// Expiry as Unix seconds.
    pub exp: i64,
    /// Cryptographically-bound approver identity (the `sub`). `None` for
    /// legacy single-approver tokens that carry no identity. When
    /// present, the approver is folded into the HMAC input, so a
    /// replayed token always resolves to the SAME approver regardless of
    /// which channel it is POSTed through — the distinctness guarantee
    /// N-of-M threshold approvals depend on (Stream
    /// `n-of-m-bind-approver-identity-and-self-approval-exclusion`).
    #[serde(default)]
    pub approver: Option<String>,
}

/// Sign approval claims into a compact URL token (no bound approver —
/// the legacy single-approver path). Equivalent to
/// [`sign_approval_token_for`] with `approver = None`.
pub fn sign_approval_token(
    approval_id: &str,
    decision: ApprovalDecision,
    exp: i64,
    secret: &str,
) -> ChannelResult<String> {
    sign_approval_token_for(approval_id, decision, exp, None, secret)
}

/// Sign approval claims into a compact URL token, optionally binding the
/// `approver` identity into the HMAC.
///
/// * `approver = None` → 4-field token `id.decision.exp.sig` (back-compat).
/// * `approver = Some(a)` → 5-field token `id.decision.exp.hex(a).sig`,
///   with `a` folded into the signed payload. The approver segment is
///   hex-encoded so identities containing `.` (e.g. email addresses)
///   never collide with the `.` field separator.
pub fn sign_approval_token_for(
    approval_id: &str,
    decision: ApprovalDecision,
    exp: i64,
    approver: Option<&str>,
    secret: &str,
) -> ChannelResult<String> {
    if secret.is_empty() {
        return Err(ChannelError::Config(
            "ARGO_CHANNEL_HMAC_SECRET must not be empty".into(),
        ));
    }
    let payload = token_payload(approval_id, decision, exp, approver);
    let sig = hmac_hex(payload.as_bytes(), secret.as_bytes())?;
    match approver {
        Some(a) => Ok(format!(
            "{}.{}.{}.{}.{}",
            approval_id,
            decision.as_token_str(),
            exp,
            hex::encode(a.as_bytes()),
            sig
        )),
        None => Ok(format!(
            "{}.{}.{}.{}",
            approval_id,
            decision.as_token_str(),
            exp,
            sig
        )),
    }
}

/// Verify a compact approval token. Accepts both the 4-field legacy form
/// and the 5-field approver-bound form; the returned
/// [`ApprovalTokenClaims::approver`] is `Some` only for the latter.
pub fn verify_approval_token(token: &str, secret: &str) -> ChannelResult<ApprovalTokenClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    let (approver_seg, sig) = match parts.len() {
        4 => (None, parts[3]),
        5 => (Some(parts[3]), parts[4]),
        _ => {
            return Err(ChannelError::Auth(
                "approval token must have 4 or 5 fields".into(),
            ))
        }
    };
    let decision = ApprovalDecision::parse_token_str(parts[1])
        .ok_or_else(|| ChannelError::Auth("invalid approval decision".into()))?;
    let exp = parts[2]
        .parse::<i64>()
        .map_err(|e| ChannelError::Auth(format!("invalid approval token exp: {e}")))?;
    let approver = match approver_seg {
        Some(seg) => Some(decode_approver(seg)?),
        None => None,
    };
    let payload = token_payload(parts[0], decision, exp, approver.as_deref());
    let expected = hmac_hex(payload.as_bytes(), secret.as_bytes())?;
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
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
        approver,
    })
}

/// Decode the hex-encoded approver segment back to its UTF-8 identity.
fn decode_approver(seg: &str) -> ChannelResult<String> {
    let bytes = hex::decode(seg)
        .map_err(|e| ChannelError::Auth(format!("invalid approver segment: {e}")))?;
    String::from_utf8(bytes)
        .map_err(|e| ChannelError::Auth(format!("approver segment not utf-8: {e}")))
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

fn token_payload(
    approval_id: &str,
    decision: ApprovalDecision,
    exp: i64,
    approver: Option<&str>,
) -> String {
    match approver {
        Some(a) => format!("{}|{}|{}|{}", approval_id, decision.as_token_str(), exp, a),
        None => format!("{}|{}|{}", approval_id, decision.as_token_str(), exp),
    }
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
    fn approver_bound_token_round_trips_and_binds_identity() {
        let exp = Utc::now().timestamp() + 60;
        // An email approver — contains a `.`, exercising the hex-encoded
        // segment that keeps the field separator unambiguous.
        let token = sign_approval_token_for(
            "act-1",
            ApprovalDecision::Approve,
            exp,
            Some("alice@corp.example"),
            "secret",
        )
        .expect("token");
        let claims = verify_approval_token(&token, "secret").expect("verify");
        assert_eq!(claims.approval_id, "act-1");
        assert_eq!(claims.approver.as_deref(), Some("alice@corp.example"));
        // The approver is in the signed payload: swapping it for another
        // identity (re-using the same signature) must NOT verify.
        let bob_seg = hex::encode(b"bob@corp.example");
        let parts: Vec<&str> = token.split('.').collect();
        let forged = format!("{}.{}.{}.{}.{}", parts[0], parts[1], parts[2], bob_seg, parts[4]);
        assert!(
            verify_approval_token(&forged, "secret").is_err(),
            "re-binding the signature to a different approver must fail"
        );
    }

    #[test]
    fn legacy_unbound_token_has_no_approver() {
        let exp = Utc::now().timestamp() + 60;
        let token =
            sign_approval_token("act-1", ApprovalDecision::Approve, exp, "secret").expect("token");
        let claims = verify_approval_token(&token, "secret").expect("verify");
        assert_eq!(claims.approver, None);
    }

    #[test]
    fn webhook_signature_checks_body_hmac() {
        let sig = hmac_hex(br#"{"ok":true}"#, b"secret").expect("sig");
        verify_webhook_signature(br#"{"ok":true}"#, &sig, "secret").expect("valid");
        assert!(verify_webhook_signature(br#"{"ok":false}"#, &sig, "secret").is_err());
    }
}
