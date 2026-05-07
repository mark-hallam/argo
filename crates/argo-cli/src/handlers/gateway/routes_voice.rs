//! Twilio Voice DTMF approval webhook route.

use crate::handlers::worker_actions::{ActionRecord, ActionState};
use crate::handlers::worker_actions_transitions::{self, LocatedAction};
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, PartialEq, Eq)]
enum VoiceDecision {
    Approve,
    Deny,
    RequestDetails,
}

struct VoiceTokenClaims {
    approval_id: String,
    action_id: String,
    operator_id: String,
}

pub(super) fn is_voice_webhook(req: &str) -> bool {
    req.starts_with("POST /twilio/voice/decision ")
        || req.starts_with("POST /twilio/voice/decision?")
}

pub(super) async fn voice_webhook_response(req: &str) -> (&'static str, String) {
    let auth_token = match env_required("TWILIO_AUTH_TOKEN") {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let Some(sig) = header_value(req, "X-Twilio-Signature") else {
        return unauthorized("missing X-Twilio-Signature");
    };
    let params = form_params(super::body_from_request(req));
    let url = public_url_for_signature(req);
    if !verify_twilio_signature(&url, &params, sig, &auth_token) {
        return unauthorized("invalid X-Twilio-Signature");
    }
    let secret = match env_required("ARGO_CHANNEL_HMAC_SECRET") {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let Some(token) = voice_token(req, &params) else {
        return bad_request("missing voice callback token");
    };
    let claims = match verify_voice_token(&token, &secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&e),
    };
    let Some(digits) = form_param(&params, "Digits") else {
        return bad_request("missing DTMF digits");
    };
    let decision = match VoiceDecision::from_digits(&digits) {
        Some(d) => d,
        None => return bad_request("unsupported DTMF digit"),
    };
    let actor = form_param(&params, "From")
        .or_else(|| form_param(&params, "Caller"))
        .unwrap_or_else(|| claims.operator_id.clone());
    match settle_voice(&claims, decision, &actor).await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

impl VoiceDecision {
    fn from_digits(digits: &str) -> Option<Self> {
        match digits.trim() {
            "1" => Some(Self::Approve),
            "2" => Some(Self::Deny),
            "9" => Some(Self::RequestDetails),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Deny => "deny",
            Self::RequestDetails => "request-details",
        }
    }
}

async fn settle_voice(
    claims: &VoiceTokenClaims,
    decision: VoiceDecision,
    actor: &str,
) -> ArgoResult<String> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let located = worker_actions_transitions::find_action(&memory, ws, &claims.approval_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no approval '{}'", claims.approval_id)))?;
    if located.record.action_id != claims.action_id {
        return Err(ArgoError::Provider("voice callback action mismatch".into()));
    }
    if located.record.state != ActionState::Pending.as_str() {
        return Ok(json!({
            "ok": true,
            "already_resolved": true,
            "approval_id": claims.approval_id,
            "state": located.record.state,
            "resolved_by": resolved_by(&located.record)
        })
        .to_string());
    }
    if !located.record.requires_approval {
        return Ok(json!({
            "ok": true,
            "already_resolved": true,
            "approval_id": claims.approval_id,
            "state": located.record.state,
            "resolved_by": "autonomous-safe"
        })
        .to_string());
    }
    let rec = match decision {
        VoiceDecision::Approve => {
            worker_actions_transitions::approve(
                &memory,
                ws,
                &claims.approval_id,
                actor,
                Some("voice approval"),
            )
            .await?
        }
        VoiceDecision::Deny => deny_action(&memory, ws, &located, actor).await?,
        VoiceDecision::RequestDetails => request_details_action(&memory, ws, &located, actor).await?,
    };
    Ok(json!({
        "ok": true,
        "approval_id": rec.action_id,
        "decision": decision.as_str(),
        "state": rec.state,
        "resolved_by": actor
    })
    .to_string())
}

async fn deny_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    located: &LocatedAction,
    actor: &str,
) -> ArgoResult<ActionRecord> {
    let current = ActionState::parse(&located.record.state).ok_or_else(|| {
        ArgoError::Provider(format!(
            "action {}: unknown current state '{}'",
            located.record.action_id, located.record.state
        ))
    })?;
    if !current.can_transition_to(ActionState::Blocked) {
        return Err(ArgoError::Provider(format!(
            "action {}: illegal transition {} -> blocked",
            located.record.action_id,
            current.as_str()
        )));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let mut next = located.record.clone();
    next.state = ActionState::Blocked.as_str().to_string();
    next.created_at = now.clone();
    next.approved_by = Some(actor.to_string());
    next.approved_at = Some(now);
    next.blocked_reason = Some(format!("denied by {actor} via voice"));
    next.result = Some(json!({"decision":"deny","source":"voice"}));
    append_action(memory, workspace, &located.vertical_id, next).await
}

async fn request_details_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    located: &LocatedAction,
    actor: &str,
) -> ArgoResult<ActionRecord> {
    let mut next = located.record.clone();
    next.created_at = chrono::Utc::now().to_rfc3339();
    next.result = Some(json!({
        "decision": "request-details",
        "source": "voice",
        "requested_by": actor
    }));
    append_action(memory, workspace, &located.vertical_id, next).await
}

async fn append_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_id: &str,
    rec: ActionRecord,
) -> ArgoResult<ActionRecord> {
    let body = serde_json::to_string(&rec)
        .map_err(|e| ArgoError::Provider(format!("serialize ActionRecord: {e}")))?;
    memory
        .append_for_vertical(workspace, MemoryFile::Memory, body, None, vertical_id)
        .await?;
    Ok(rec)
}

fn resolved_by(record: &ActionRecord) -> String {
    record
        .approved_by
        .clone()
        .or_else(|| record.blocked_reason.clone())
        .unwrap_or_else(|| format!("state {}", record.state))
}

fn verify_twilio_signature(
    url: &str,
    params: &[(String, String)],
    signature: &str,
    auth_token: &str,
) -> bool {
    let mut sorted = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut data = url.to_string();
    for (k, v) in sorted {
        data.push_str(&k);
        data.push_str(&v);
    }
    let expected = base64_encode(&hmac_sha1(data.as_bytes(), auth_token.as_bytes()));
    constant_time_eq(expected.as_bytes(), signature.trim().as_bytes())
}

fn public_url_for_signature(req: &str) -> String {
    let target = request_target(req).unwrap_or("/twilio/voice/decision");
    if let Ok(base) = std::env::var("ARGO_TWILIO_VOICE_DECISION_URL") {
        if !base.is_empty() {
            return match target.split_once('?') {
                Some((_, q)) if !q.is_empty() => {
                    let sep = if base.contains('?') { '&' } else { '?' };
                    format!("{base}{sep}{q}")
                }
                _ => base,
            };
        }
    }
    let proto = header_value(req, "X-Forwarded-Proto").unwrap_or("http");
    let host = header_value(req, "X-Forwarded-Host")
        .or_else(|| header_value(req, "Host"))
        .unwrap_or("localhost");
    format!("{proto}://{host}{target}")
}

fn voice_token(req: &str, params: &[(String, String)]) -> Option<String> {
    request_target(req)
        .and_then(|target| query_value(target, "vt"))
        .or_else(|| form_param(params, "vt"))
}

fn verify_voice_token(token: &str, secret: &str) -> Result<VoiceTokenClaims, String> {
    let (payload_hex, sig) = token
        .rsplit_once('.')
        .ok_or_else(|| "voice callback token must have payload.signature".to_string())?;
    let payload_bytes = hex::decode(payload_hex)
        .map_err(|e| format!("voice callback token payload hex: {e}"))?;
    let payload = std::str::from_utf8(&payload_bytes)
        .map_err(|e| format!("voice callback token payload utf8: {e}"))?;
    let expected = hmac_hex(payload.as_bytes(), secret.as_bytes());
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
        return Err("voice callback token signature mismatch".into());
    }
    let parts: Vec<&str> = payload.split('|').collect();
    if parts.len() != 5 {
        return Err("voice callback token payload must have 5 fields".into());
    }
    let exp = parts[3]
        .parse::<i64>()
        .map_err(|e| format!("voice callback token exp: {e}"))?;
    if chrono::Utc::now().timestamp() > exp {
        return Err("voice callback token expired".into());
    }
    Ok(VoiceTokenClaims {
        approval_id: parts[0].to_string(),
        action_id: parts[1].to_string(),
        operator_id: parts[2].to_string(),
    })
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    let head = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    head.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

fn request_target(req: &str) -> Option<&str> { req.lines().next()?.split_whitespace().nth(1) }

fn query_value(target: &str, key: &str) -> Option<String> {
    let query = target.split_once('?')?.1;
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (percent_decode(k).as_deref() == Some(key)).then(|| percent_decode(v)).flatten()
    })
}

fn form_params(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .filter(|part| !part.is_empty())
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            Some((percent_decode(k)?, percent_decode(v)?))
        })
        .collect()
}

fn form_param(params: &[(String, String)], key: &str) -> Option<String> { params.iter().find_map(|(k, v)| (k == key).then(|| v.clone())) }

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

fn env_required(name: &str) -> Result<String, String> {
    let value = std::env::var(name).map_err(|_| format!("missing {name}"))?;
    if value.is_empty() {
        Err(format!("{name} must not be empty"))
    } else {
        Ok(value)
    }
}

fn hmac_hex(data: &[u8], secret: &[u8]) -> String {
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

fn hmac_sha1(data: &[u8], secret: &[u8]) -> [u8; 20] {
    const BLOCK: usize = 64;
    let mut key = [0_u8; BLOCK];
    if secret.len() > BLOCK {
        key[..20].copy_from_slice(&sha1_digest(secret));
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut ipad = [0x36_u8; BLOCK];
    let mut opad = [0x5c_u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Vec::with_capacity(BLOCK + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner = sha1_digest(&inner);
    let mut outer = Vec::with_capacity(BLOCK + inner.len());
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner);
    sha1_digest(&outer)
}

fn sha1_digest(data: &[u8]) -> [u8; 20] {
    let mut h0 = 0x6745_2301_u32;
    let mut h1 = 0xEFCD_AB89_u32;
    let mut h2 = 0x98BA_DCFE_u32;
    let mut h3 = 0x1032_5476_u32;
    let mut h4 = 0xC3D2_E1F0_u32;
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0_u32; 80];
        for (slot, bytes) in w.iter_mut().take(16).zip(chunk.chunks_exact(4)) {
            *slot = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    let mut out = [0_u8; 20];
    for (slot, word) in out.chunks_exact_mut(4).zip([h0, h1, h2, h3, h4]) {
        slot.copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(b2 & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0_u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

fn bad_request(msg: &str) -> (&'static str, String) { ("400 Bad Request", format!("{{\"error\":\"{msg}\"}}")) }
fn unauthorized(msg: &str) -> (&'static str, String) { ("401 Unauthorized", format!("{{\"error\":\"{msg}\"}}")) }
fn server_error(msg: &str) -> (&'static str, String) { ("500 Internal Server Error", format!("{{\"error\":\"{msg}\"}}")) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twilio_signature_accepts_valid_and_rejects_forged() {
        let url = "https://gw.example.com/twilio/voice/decision?vt=abc";
        let params = vec![("Digits".to_string(), "1".to_string())];
        let sig = base64_encode(&hmac_sha1(format!("{url}Digits1").as_bytes(), b"auth"));
        assert!(verify_twilio_signature(url, &params, &sig, "auth"));
        let forged = vec![("Digits".to_string(), "2".to_string())];
        assert!(!verify_twilio_signature(url, &forged, &sig, "auth"));
    }
}
