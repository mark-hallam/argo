//! Channel webhook + web inbox routes — central dispatcher and
//! shared primitives. Per-channel handlers live in sibling files
//! (`routes_channels_email`, `routes_channels_slack`, `routes_channels_discord`,
//! `routes_channels_webinbox`, `routes_channels_push`,
//! `routes_channels_sms`, `routes_channels_whatsapp`, `routes_channels_voice`).
//!
//! Adding a new channel: implement `is_X` + `X_response` in a new
//! `routes_channels_<channel>.rs`, then add a branch to
//! [`is_email_webhook`] / [`email_webhook_response`].

use super::routes_channels_discord::{discord_decision_response, is_discord_decision};
use super::routes_channels_email::handle_email_webhook;
use super::routes_channels_imessage::{imessage_webhook_response, is_imessage_webhook};
use super::routes_channels_push::{is_push, push_response};
use super::routes_channels_slack::{is_slack_webhook, slack_webhook_response};
use super::routes_channels_sms::{is_sms_webhook, sms_webhook_response};
use super::routes_channels_voice::{is_voice_webhook, voice_webhook_response};
use super::routes_channels_webinbox::{is_webinbox, webinbox_response};
use super::routes_channels_whatsapp::{is_whatsapp_webhook, whatsapp_webhook_response};
use crate::handlers::worker_actions::{ActionRecord, ActionState};
use crate::handlers::worker_actions_transitions;
use argo_audit::{AuditKind, AuditSink};
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Decision {
    Approve,
    Reject,
}

pub(super) struct TokenClaims {
    pub(super) approval_id: String,
    pub(super) decision: Decision,
    /// Cryptographically-bound approver identity, when the token carries
    /// one (5-field token). `None` for legacy single-approver tokens.
    /// When present this is the authoritative actor at settle time — the
    /// channel-supplied actor string is NOT trusted (it is hardcoded for
    /// push, read from a spoofable From for email, from the payload for
    /// slack). A replayed token therefore always resolves to the same
    /// approver, which is what makes N-of-M distinctness real.
    pub(super) approver: Option<String>,
}

/// True if the request is any channel webhook or web inbox route
/// (Postmark email, Slack, web inbox UI, or push device/decision).
/// Name retained for backward-compatibility with the dispatch loop.
pub(super) fn is_email_webhook(req: &str) -> bool {
    req.starts_with("POST /channels/email/webhook ")
        || req.starts_with("POST /channels/email/webhook?")
        || is_slack_webhook(req)
        || is_discord_decision(req)
        || is_webinbox(req)
        || is_push(req)
        || is_sms_webhook(req)
        || is_whatsapp_webhook(req)
        || is_imessage_webhook(req)
        || is_voice_webhook(req)
}

/// Dispatch to the matching channel handler. Falls through to
/// email (Postmark) when no other channel claims the request.
pub(super) async fn email_webhook_response(req: &str) -> (&'static str, String) {
    if is_slack_webhook(req) {
        return slack_webhook_response(req).await;
    }
    if is_discord_decision(req) {
        return discord_decision_response(req).await;
    }
    if is_webinbox(req) {
        return webinbox_response(req).await;
    }
    if is_push(req) {
        return push_response(req).await;
    }
    if is_sms_webhook(req) {
        return sms_webhook_response(req).await;
    }
    if is_whatsapp_webhook(req) {
        return whatsapp_webhook_response(req).await;
    }
    if is_imessage_webhook(req) {
        return imessage_webhook_response(req).await;
    }
    if is_voice_webhook(req) {
        return voice_webhook_response(req).await;
    }
    handle_email_webhook(req).await
}

pub(super) async fn settle(
    claims: &TokenClaims,
    actor: &str,
    note: Option<&str>,
    source: &str,
) -> ArgoResult<String> {
    let (memory, ws, store) =
        worker_actions_transitions::open_workspace_memory_with_store().await?;
    let located = worker_actions_transitions::find_action(&memory, ws, &claims.approval_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no approval '{}'", claims.approval_id)))?;
    if located.record.state != ActionState::Pending.as_str() {
        // Late replies hit this branch: don't emit a second
        // `approval.replied` row per the brief's hidden constraint
        // (first reply wins).
        return Ok(json!({"ok":true,"already_resolved":true,"approval_id":&claims.approval_id,"state":located.record.state,"resolved_by":resolved_by(&located.record)}).to_string());
    }
    if !located.record.requires_approval {
        return Ok(json!({"ok":true,"already_resolved":true,"approval_id":&claims.approval_id,"state":located.record.state,"resolved_by":"autonomous-safe"}).to_string());
    }
    // Identity binding: when the token carries a cryptographically-bound
    // approver it is the authoritative actor — NOT the channel-supplied
    // string (push hardcodes `push-operator`, email reads a spoofable
    // From, slack reads the payload). A token replayed across channels
    // therefore always resolves to the SAME approver, which is what makes
    // the N-of-M distinct-approver count trustworthy. Legacy single-
    // approver tokens carry no binding and fall back to the channel actor.
    let actor = claims.approver.as_deref().unwrap_or(actor);
    // Emit `AuditKind::ApprovalReplied` BEFORE handing off to the
    // gate (audit finding P0-2). The action is still `Pending` here
    // — late replies short-circuit above. The workspace tenant id
    // stamps the row alongside the same tenant that owns the
    // workspace's other audit rows.
    let tenant = argo_config::workspace_tenant_id(&std::env::current_dir().unwrap_or_default());
    let audit = AuditSink::new_with_tenant(std::sync::Arc::clone(&store), tenant);
    let payload = json!({
        "action_id": &claims.approval_id,
        "channel": source,
        "replier": actor,
        "reply_kind": decision_str(claims.decision),
        "reply_body": note,
        "replied_at": chrono::Utc::now().to_rfc3339(),
    });
    if let Err(e) = audit
        .write(None, AuditKind::ApprovalReplied, payload, None)
        .await
    {
        tracing::warn!(
            approval_id = %claims.approval_id,
            channel = %source,
            error = %e,
            "approval.replied audit write failed; continuing to gate",
        );
    }
    // N-of-M threshold approvals (Stream `n-of-m-multi-step-approval`):
    // when the approval request declares a threshold rule on its
    // payload, record this approver's VOTE and only finalize once the
    // quorum of distinct approvals is met (or any reject denies).
    // Absence of the rule = single-approver, falling through unchanged.
    if let Some(rule) =
        argo_identity::auto_approval::ThresholdRule::from_payload(&located.record.payload)
    {
        use super::routes_approvals_threshold::{self, ThresholdOutcome};
        // N-of-M is a separation-of-duties control, so every COUNTED vote
        // must come from a cryptographically-bound, named approver. An
        // unbound (legacy 4-field) token cannot satisfy a threshold vote:
        // otherwise one principal could POST the same token to several
        // channels under different actor strings and forge a quorum (the
        // exact #2747 exploit this brief closes).
        if claims.approver.is_none() {
            return Ok(json!({
                "ok": false,
                "approval_id": &claims.approval_id,
                "error": "threshold approval requires an identity-bound approval token",
            })
            .to_string());
        }
        let outcome = routes_approvals_threshold::record_threshold_vote(
            &audit,
            store.pool(),
            &located,
            actor,
            claims.decision,
            source,
            &rule,
        )
        .await?;
        return Ok(match outcome {
            ThresholdOutcome::Unauthorized => json!({
                "ok": false,
                "approval_id": &claims.approval_id,
                "error": "approver not authorized for this threshold approval",
            })
            .to_string(),
            ThresholdOutcome::Held { distinct, threshold } => json!({
                "ok": true,
                "held": true,
                "approval_id": &claims.approval_id,
                "decision": decision_str(claims.decision),
                "approvals": distinct,
                "threshold": threshold,
                "state": located.record.state,
            })
            .to_string(),
            ThresholdOutcome::FinalizeReject => {
                let rec = reject_action(&memory, ws, &located, actor, note, source).await?;
                if let Err(e) = audit
                    .write(
                        None,
                        AuditKind::ApprovalDeclined,
                        json!({
                            "action_id": &claims.approval_id,
                            "declined_by": actor,
                            "via": "n_of_m_reject",
                            "source": source,
                        }),
                        None,
                    )
                    .await
                {
                    tracing::warn!(approval_id = %claims.approval_id, error = %e, "approval.declined audit write failed");
                }
                json!({"ok":true,"approval_id":rec.action_id,"decision":"reject","state":rec.state,"resolved_by":actor,"threshold_met":false}).to_string()
            }
            ThresholdOutcome::FinalizeApprove => {
                let rec =
                    worker_actions_transitions::approve(&memory, ws, &claims.approval_id, actor, note)
                        .await?;
                if let Err(e) = audit
                    .write(
                        None,
                        AuditKind::ApprovalGranted,
                        json!({
                            "action_id": &claims.approval_id,
                            "granted_by": actor,
                            "via": "n_of_m_threshold",
                            "threshold": rule.threshold,
                            "source": source,
                        }),
                        None,
                    )
                    .await
                {
                    tracing::warn!(approval_id = %claims.approval_id, error = %e, "approval.granted audit write failed");
                }
                json!({"ok":true,"approval_id":rec.action_id,"decision":"approve","state":rec.state,"resolved_by":actor,"threshold_met":true}).to_string()
            }
        });
    }
    let rec = match claims.decision {
        Decision::Approve => {
            worker_actions_transitions::approve(&memory, ws, &claims.approval_id, actor, note)
                .await?
        }
        Decision::Reject => reject_action(&memory, ws, &located, actor, note, source).await?,
    };
    Ok(json!({"ok":true,"approval_id":rec.action_id,"decision":decision_str(claims.decision),"state":rec.state,"resolved_by":actor}).to_string())
}

async fn reject_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    located: &worker_actions_transitions::LocatedAction,
    actor: &str,
    note: Option<&str>,
    source: &str,
) -> ArgoResult<ActionRecord> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut next = located.record.clone();
    next.state = ActionState::Blocked.as_str().to_string();
    next.created_at = now.clone();
    next.approved_by = Some(actor.to_string());
    next.approved_at = Some(now);
    next.blocked_reason = Some(format!("rejected by {actor} via {source}"));
    next.result = Some(json!({"decision":"reject","source":source,"note":note}));
    let body = serde_json::to_string(&next)
        .map_err(|e| ArgoError::Provider(format!("serialize ActionRecord: {e}")))?;
    memory
        .append_for_vertical(
            workspace,
            MemoryFile::Memory,
            body,
            None,
            &located.vertical_id,
        )
        .await?;
    Ok(next)
}

fn resolved_by(record: &ActionRecord) -> String {
    record
        .approved_by
        .clone()
        .or_else(|| {
            record
                .blocked_reason
                .as_ref()
                .map(|r| r.trim_start_matches("rejected by ").to_string())
        })
        .unwrap_or_else(|| format!("state {}", record.state))
}

pub(super) fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    let head = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    head.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

pub(super) fn percent_decode(s: &str) -> Option<String> {
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

pub(super) fn clean_token(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | '('))
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | '('))
        .next()
        .unwrap_or("")
        .to_string()
}

pub(super) fn verify_token(token: &str, secret: &str) -> Result<TokenClaims, String> {
    let parts: Vec<&str> = token.split('.').collect();
    // 4-field = legacy single-approver token; 5-field = approver-bound
    // (the hex-encoded approver sits in slot 3, signature shifts to 4).
    let (approver_seg, sig) = match parts.len() {
        4 => (None, parts[3]),
        5 => (Some(parts[3]), parts[4]),
        _ => return Err("approval token must have 4 or 5 fields".into()),
    };
    let decision = match parts[1] {
        "approve" => Decision::Approve,
        "reject" => Decision::Reject,
        _ => return Err("invalid approval decision".into()),
    };
    let exp = parts[2]
        .parse::<i64>()
        .map_err(|e| format!("invalid approval token exp: {e}"))?;
    if chrono::Utc::now().timestamp() > exp {
        return Err("approval token expired".into());
    }
    let approver = match approver_seg {
        Some(seg) => {
            let bytes =
                hex::decode(seg).map_err(|e| format!("invalid approver segment: {e}"))?;
            Some(String::from_utf8(bytes).map_err(|e| format!("approver segment not utf-8: {e}"))?)
        }
        None => None,
    };
    // The bound approver is folded into the signed payload so it cannot
    // be swapped for another identity without re-minting (which needs
    // the secret).
    let payload = match approver.as_deref() {
        Some(a) => format!("{}|{}|{exp}|{a}", parts[0], parts[1]),
        None => format!("{}|{}|{exp}", parts[0], parts[1]),
    };
    let expected = hmac_hex(payload.as_bytes(), secret.as_bytes());
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
        return Err("approval token signature mismatch".into());
    }
    Ok(TokenClaims {
        approval_id: parts[0].to_string(),
        decision,
        approver,
    })
}

pub(super) fn hmac_secret() -> Result<String, String> {
    // tenant-unscoped: gateway process-level secret (no per-request tenant context for this slot)
    let secret = argo_config::cred::get_unscoped("ARGO_CHANNEL_HMAC_SECRET")
        .map_err(|_| "missing ARGO_CHANNEL_HMAC_SECRET".to_string())?;
    if secret.is_empty() {
        Err("ARGO_CHANNEL_HMAC_SECRET must not be empty".into())
    } else {
        Ok(secret)
    }
}

pub(super) fn hmac_hex(data: &[u8], secret: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut key = [0_u8; BLOCK];
    if secret.len() > BLOCK {
        let digest = Sha256::digest(secret);
        key[..digest.len()].copy_from_slice(&digest);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut ipad = [0x36_u8; BLOCK];
    let mut opad = [0x5c_u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner);
    hex::encode(outer.finalize())
}

pub(super) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0_u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

fn decision_str(d: Decision) -> &'static str {
    match d {
        Decision::Approve => "approve",
        Decision::Reject => "reject",
    }
}

pub(super) fn bad_request(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", format!("{{\"error\":\"{msg}\"}}"))
}
pub(super) fn unauthorized(msg: &str) -> (&'static str, String) {
    ("401 Unauthorized", format!("{{\"error\":\"{msg}\"}}"))
}
pub(super) fn server_error(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        format!("{{\"error\":\"{msg}\"}}"),
    )
}
/// A missing/empty server-side channel secret is a config-absence, not a
/// request fault: an unauthenticated caller must NOT receive a `500` that
/// names the unset env var (wrong status class + minor info-disclosure,
/// e2e R3-F3/R1-F1). Map it to a generic `503` instead — the message is
/// deliberately fixed so no caller can accidentally echo a secret's name.
pub(super) fn not_configured() -> (&'static str, String) {
    (
        "503 Service Unavailable",
        "{\"error\":\"channel approvals not configured\"}".to_string(),
    )
}
