//! Email (Postmark) approval webhook decision route.
//!
//! Sibling of [`super::routes_channels`]. Owns the Postmark-side
//! token extraction (HTTP query, JSON body field, or text-body
//! pattern) and the actor/note shape Postmark posts back. Shared
//! settle/HMAC/helpers live in the parent file.

use super::routes_channels::{
    bad_request, clean_token, constant_time_eq, header_value, hmac_hex, hmac_secret,
    percent_decode, server_error, settle, unauthorized, verify_token, Decision,
};
use argo_channels::email::{classify_postmark_record, inbound_event_from_payload, PostmarkRecord};
use argo_channels::token::ApprovalDecision;
use argo_channels::ChannelId;
use serde_json::{json, Value};

pub(super) async fn handle_email_webhook(req: &str) -> (&'static str, String) {
    let secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let body = super::body_from_request(req);
    let Some(sig) = header_value(req, "X-Postmark-Signature") else {
        return unauthorized("missing X-Postmark-Signature");
    };
    if !constant_time_eq(
        hmac_hex(body.as_bytes(), secret.as_bytes()).as_bytes(),
        sig.as_bytes(),
    ) {
        return unauthorized("invalid X-Postmark-Signature");
    }
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse JSON body: {e}")),
    };
    // Branch on Postmark's `RecordType`. Delivery-event records
    // (`Bounce`/`SpamComplaint`/`Open`/`Click`/...) are acknowledged +
    // audited — never mis-parsed as an approval token (which previously
    // returned a spurious 400). Inbound operator replies continue
    // through the token-verify + settle path.
    match classify_postmark_record(&parsed) {
        PostmarkRecord::Event(kind) => acknowledge_event(&kind, &parsed).await,
        PostmarkRecord::InboundReply => settle_inbound_reply(req, &parsed, &secret).await,
    }
}

/// Settle a genuine inbound operator reply: extract + verify the
/// approval token, build the structured [`InboundEvent`] through the
/// channel library (single source of truth for actor/note), then
/// settle on the shared path SMS/iMessage use.
async fn settle_inbound_reply(req: &str, parsed: &Value, secret: &str) -> (&'static str, String) {
    let Some(token) = extract_token(req, parsed) else {
        return bad_request("missing approval token");
    };
    let claims = match verify_token(&token, secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&e),
    };
    let decision = match claims.decision {
        Decision::Approve => ApprovalDecision::Approve,
        Decision::Reject => ApprovalDecision::Reject,
    };
    let event = inbound_event_from_payload(
        ChannelId::new("email"),
        parsed.clone(),
        claims.approval_id.clone(),
        decision,
    );
    match settle(&claims, &event.actor, event.note.as_deref(), "email").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

/// Acknowledge a Postmark delivery-event record with a 200, writing a
/// best-effort audit row. The acknowledgement never fails on a missing
/// workspace/store — a delivery event is informational, so we always
/// return 200 even if the audit write can't land.
async fn acknowledge_event(record_type: &str, payload: &Value) -> (&'static str, String) {
    write_event_audit(record_type, payload).await;
    ("200 OK", event_ack_body(record_type, payload))
}

fn event_ack_body(record_type: &str, payload: &Value) -> String {
    json!({
        "ok": true,
        "acknowledged": true,
        "record_type": record_type,
        "message_id": event_message_id(payload),
    })
    .to_string()
}

fn event_message_id(payload: &Value) -> Option<&str> {
    ["MessageID", "MessageId"]
        .iter()
        .find_map(|k| payload.get(*k).and_then(Value::as_str))
}

/// Write one best-effort audit row for an acknowledged delivery event.
/// Reuses the same workspace-store open path `settle` uses; any
/// failure is logged and swallowed so the webhook still returns 200.
async fn write_event_audit(record_type: &str, payload: &Value) {
    use argo_audit::{AuditKind, AuditSink};
    let store = match crate::handlers::worker_actions_transitions::open_workspace_memory_with_store(
    )
    .await
    {
        Ok((_, _, store)) => store,
        Err(e) => {
            tracing::warn!(
                record_type,
                error = %e,
                "email delivery-event acknowledged without audit row (workspace open failed)",
            );
            return;
        }
    };
    let tenant = argo_config::workspace_tenant_id(&std::env::current_dir().unwrap_or_default());
    let audit = AuditSink::new_with_tenant(store, tenant);
    let row = json!({
        "channel": "email",
        "record_type": record_type,
        "message_id": event_message_id(payload),
        "acknowledged_at": chrono::Utc::now().to_rfc3339(),
    });
    if let Err(e) = audit.write(None, AuditKind::HttpRequest, row, None).await {
        tracing::warn!(
            record_type,
            error = %e,
            "email delivery-event audit write failed; acknowledged anyway",
        );
    }
}

pub(super) fn extract_token(req: &str, payload: &Value) -> Option<String> {
    request_query_token(req)
        .or_else(|| json_token(payload))
        .or_else(|| text_token(payload))
}

fn request_query_token(req: &str) -> Option<String> {
    req.strip_prefix("POST ")?
        .split_once(' ')?
        .0
        .split_once('?')?
        .1
        .split('&')
        .find_map(|kv| kv.strip_prefix("token=").map(clean_token))
}

fn json_token(payload: &Value) -> Option<String> {
    ["token", "Token", "approval_token", "ApprovalToken"]
        .iter()
        .find_map(|k| payload.get(*k).and_then(Value::as_str).map(clean_token))
}

fn text_token(payload: &Value) -> Option<String> {
    ["StrippedTextReply", "TextBody", "HtmlBody", "Subject"]
        .iter()
        .filter_map(|k| payload.get(*k).and_then(Value::as_str))
        .find_map(find_token_in_text)
}

fn find_token_in_text(s: &str) -> Option<String> {
    ["approval_token=", "token="]
        .iter()
        .find_map(|m| s.split_once(m).map(|(_, rest)| clean_token(rest)))
}

pub(super) fn token_from_url(url: &str) -> Option<String> {
    url.split_once('?')?
        .1
        .split('&')
        .find_map(|kv| kv.strip_prefix("token=").and_then(percent_decode))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_token_finds_token_in_email_payload() {
        let payload = json!({"TextBody":"ok\napproval_token=act.approve.999.sig\nthanks"});
        assert_eq!(
            extract_token("POST /channels/email/webhook HTTP/1.1\r\n\r\n{}", &payload),
            Some("act.approve.999.sig".into())
        );
    }

    #[test]
    fn record_type_routes_events_to_acknowledgement() {
        // Event records classify to the acknowledge path, never the
        // token path — this is the branch `handle_email_webhook` takes.
        let bounce = json!({"RecordType": "Bounce", "Email": "x@y.z", "MessageID": "m-1"});
        assert_eq!(
            classify_postmark_record(&bounce),
            PostmarkRecord::Event("Bounce".into()),
        );
        let reply = json!({"From": "op@x.io", "TextBody": "approve token=act.approve.9.s"});
        assert_eq!(classify_postmark_record(&reply), PostmarkRecord::InboundReply);
    }

    #[test]
    fn event_ack_body_is_200_shaped_and_carries_record_type() {
        let bounce = json!({"RecordType": "Bounce", "MessageID": "m-42"});
        let body: Value = serde_json::from_str(&event_ack_body("Bounce", &bounce)).expect("json");
        assert_eq!(body["ok"], true);
        assert_eq!(body["acknowledged"], true);
        assert_eq!(body["record_type"], "Bounce");
        assert_eq!(body["message_id"], "m-42");
    }
}
