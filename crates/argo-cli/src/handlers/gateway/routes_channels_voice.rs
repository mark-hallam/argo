//! Twilio Voice DTMF approval webhook route.

use crate::handlers::worker_actions::{ActionRecord, ActionState};
use crate::handlers::worker_actions_transitions::{self, LocatedAction};
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde_json::json;

use super::routes_channels::{
    bad_request, header_value, hmac_secret, percent_decode, server_error, unauthorized,
};
use super::routes_voice_auth::{
    public_url_for_signature, verify_twilio_signature, verify_voice_token, voice_token,
    VoiceTokenClaims,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum VoiceDecision { Approve, Deny, RequestDetails }

pub(super) fn is_voice_webhook(req: &str) -> bool {
    req.starts_with("POST /twilio/voice/decision ")
        || req.starts_with("POST /twilio/voice/decision?")
}

pub(super) async fn voice_webhook_response(req: &str) -> (&'static str, String) {
    let auth_token = match twilio_auth_token() {
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
    let secret = match hmac_secret() {
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
        match digits.trim() { "1" => Some(Self::Approve), "2" => Some(Self::Deny), "9" => Some(Self::RequestDetails), _ => None }
    }

    fn as_str(self) -> &'static str {
        match self { Self::Approve => "approve", Self::Deny => "deny", Self::RequestDetails => "request-details" }
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
        return Ok(json!({"ok":true,"already_resolved":true,"approval_id":claims.approval_id,"state":located.record.state,"resolved_by":resolved_by(&located.record)}).to_string());
    }
    if !located.record.requires_approval {
        return Ok(json!({"ok":true,"already_resolved":true,"approval_id":claims.approval_id,"state":located.record.state,"resolved_by":"autonomous-safe"}).to_string());
    }
    let rec = match decision {
        VoiceDecision::Approve => worker_actions_transitions::approve(&memory, ws, &claims.approval_id, actor, Some("voice approval")).await?,
        VoiceDecision::Deny => deny_action(&memory, ws, &located, actor).await?,
        VoiceDecision::RequestDetails => request_details_action(&memory, ws, &located, actor).await?,
    };
    Ok(json!({"ok":true,"approval_id":rec.action_id,"decision":decision.as_str(),"state":rec.state,"resolved_by":actor}).to_string())
}

async fn deny_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    located: &LocatedAction,
    actor: &str,
) -> ArgoResult<ActionRecord> {
    let current = ActionState::parse(&located.record.state).ok_or_else(|| {
        ArgoError::Provider(format!("action {}: unknown current state '{}'", located.record.action_id, located.record.state))
    })?;
    if !current.can_transition_to(ActionState::Blocked) {
        return Err(ArgoError::Provider(format!("action {}: illegal transition {} -> blocked", located.record.action_id, current.as_str())));
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
    next.result = Some(json!({"decision":"request-details","source":"voice","requested_by":actor}));
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
    record.approved_by.clone().or_else(|| record.blocked_reason.clone()).unwrap_or_else(|| format!("state {}", record.state))
}

pub(super) fn request_target(req: &str) -> Option<&str> { req.lines().next()?.split_whitespace().nth(1) }

pub(super) fn query_value(target: &str, key: &str) -> Option<String> {
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

pub(super) fn form_param(params: &[(String, String)], key: &str) -> Option<String> { params.iter().find_map(|(k, v)| (k == key).then(|| v.clone())) }

fn twilio_auth_token() -> Result<String, String> {
    argo_config::cred::get("TWILIO_AUTH_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| "missing TWILIO_AUTH_TOKEN".to_string())
}

#[cfg(test)]
#[path = "routes_channels_voice_tests.rs"]
mod tests;
