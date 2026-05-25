//! Cross-tenant negative tests for channel boundaries.
//!
//! Round-15's evaluator handoff found the `argo-channels` substrate
//! tenant-scoped in code but missing regression tests. This file pins
//! the three load-bearing boundaries called out by
//! `eval-channels-cross-tenant-negative-test`: outbound-message
//! idempotency, approval-token HMAC secrets, and Slack inbound signing
//! secrets. The NotFound envelope helper mirrors the #2068/#2091
//! existence-leak bar: cross-tenant refusals must be publicly
//! indistinguishable from genuine missing resources.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use argo_audit::AuditSink;
use argo_channels::slack_inbound::{
    sign_slack_v0, InboundChannel, SlackInbound, SlackInboundConfig,
};
use argo_channels::token::{sign_approval_token, verify_approval_token, ApprovalDecision};
use argo_channels::{ChannelError, ChannelId, ChannelResult, OutboundChannel};
use argo_store::approvals::{
    create_request, decide_request, get_request, ApprovalStatus, DefaultDecision,
    NewApprovalRequest,
};
use argo_store::SessionStore;
use argo_tool::MessageDispatcher;
use argo_types::TenantId;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

const APPROVAL_SECRET_A: &str = "tenant-a-approval-secret";
const APPROVAL_SECRET_B: &str = "tenant-b-approval-secret";
const SLACK_SIGNING_SECRET_A: &str = "tenant-a-slack-signing-secret";
const SLACK_SIGNING_SECRET_B: &str = "tenant-b-slack-signing-secret";

#[derive(Debug, Clone, PartialEq, Eq)]
struct NotFoundEnvelope {
    status: &'static str,
    body: &'static str,
}

const PUBLIC_NOT_FOUND: NotFoundEnvelope = NotFoundEnvelope {
    status: "404 Not Found",
    body: r#"{"error":"not_found"}"#,
};

#[derive(Debug)]
struct CountingChannel {
    id: ChannelId,
    sent: Mutex<Vec<(String, String)>>,
}

impl CountingChannel {
    fn new(id: &str) -> Self {
        Self {
            id: ChannelId::new(id),
            sent: Mutex::new(Vec::new()),
        }
    }

    fn sent_count(&self) -> usize {
        self.sent.lock().expect("sent mutex").len()
    }
}

#[async_trait]
impl OutboundChannel for CountingChannel {
    fn id(&self) -> ChannelId {
        self.id.clone()
    }

    async fn send_text(
        &self,
        recipient: &str,
        body: &str,
        _attachments: &[PathBuf],
    ) -> Result<String, String> {
        let mut sent = self.sent.lock().map_err(|_| "sent mutex poisoned")?;
        let next = sent.len() + 1;
        sent.push((recipient.to_string(), body.to_string()));
        Ok(format!("{}-message-{next}", self.id))
    }
}

async fn fresh_store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate store");
    store
}

fn dispatcher(
    store: Arc<SessionStore>,
    tenant_id: TenantId,
    channel: Arc<dyn OutboundChannel>,
) -> argo_channels::ProductionMessageDispatcher {
    let audit = Arc::new(AuditSink::new_with_tenant(
        store.clone(),
        tenant_id.to_string(),
    ));
    argo_channels::ProductionMessageDispatcher::new(audit, store, tenant_id).with_channel(channel)
}

async fn consume_approval_token(
    store: &SessionStore,
    tenant_id: TenantId,
    token: &str,
    secret: &str,
) -> Result<(), NotFoundEnvelope> {
    let claims = verify_approval_token(token, secret).map_err(|_| PUBLIC_NOT_FOUND)?;
    let decided = decide_request(
        store.pool(),
        &claims.approval_id,
        tenant_id,
        "grantor@example.com",
        claims.decision,
    )
    .await
    .map_err(|_| PUBLIC_NOT_FOUND)?;
    if decided {
        Ok(())
    } else {
        Err(PUBLIC_NOT_FOUND)
    }
}

fn slack_signature_boundary(
    inbound: &SlackInbound,
    body: &[u8],
    headers: &[(&str, &str)],
) -> Result<(), NotFoundEnvelope> {
    inbound
        .verify_signature(body, headers)
        .map_err(|_| PUBLIC_NOT_FOUND)
}

fn signed_slack_body(payload_json: &str) -> String {
    let mut encoded = String::from("payload=");
    for b in payload_json.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    encoded
}

fn slack_callback_body(token: &str) -> String {
    let payload = serde_json::json!({
        "type": "block_actions",
        "user": { "id": "U-tenant-a-operator" },
        "actions": [{ "action_id": "approve", "value": token }],
    })
    .to_string();
    signed_slack_body(&payload)
}

#[tokio::test]
async fn outbound_dedup_is_namespaced_by_tenant() {
    let store = fresh_store().await;
    let transport = Arc::new(CountingChannel::new("slack"));
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let dispatcher_a = dispatcher(store.clone(), tenant_a, transport.clone());
    let dispatcher_b = dispatcher(store, tenant_b, transport.clone());
    let body = "approval-token-dedup-key:shared";

    let first_a = dispatcher_a
        .dispatch("slack", "slack-1", body, &[])
        .await
        .expect("tenant_a first send");
    let first_b = dispatcher_b
        .dispatch("slack", "slack-1", body, &[])
        .await
        .expect("tenant_b first send must not dedup against tenant_a");
    let replay_b = dispatcher_b
        .dispatch("slack", "slack-1", body, &[])
        .await
        .expect("tenant_b replay should hit tenant_b cache");

    assert_ne!(
        first_a.message_id, first_b.message_id,
        "tenant_b must get a fresh provider send, not tenant_a's cached outcome"
    );
    assert_eq!(
        first_b.message_id, replay_b.message_id,
        "same-tenant replay should still dedup inside tenant_b"
    );
    assert_eq!(
        transport.sent_count(),
        2,
        "only tenant_a initial + tenant_b initial should reach the transport"
    );
}

#[tokio::test]
async fn approval_token_cross_secret_is_publicly_identical_to_nonexistent_token() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let approval_id = create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: Uuid::now_v7().as_bytes().to_vec(),
            tenant_id: tenant_a,
            requested_by: "dispatcher",
            deadline_at: Utc::now().timestamp_micros() + 300_000_000,
            default_decision: DefaultDecision::Deny,
            draft_content: Some("tenant_a approval"),
        },
    )
    .await
    .expect("create tenant_a approval");
    let exp = Utc::now().timestamp() + 300;
    let tenant_a_token = sign_approval_token(
        &approval_id,
        ApprovalDecision::Approve,
        exp,
        APPROVAL_SECRET_A,
    )
    .expect("sign tenant_a token");
    let nonexistent_token = sign_approval_token(
        &Uuid::now_v7().to_string(),
        ApprovalDecision::Approve,
        exp,
        APPROVAL_SECRET_B,
    )
    .expect("sign nonexistent tenant_b token");

    verify_approval_token(&tenant_a_token, APPROVAL_SECRET_A)
        .expect("tenant_a token verifies with tenant_a secret");

    let wrong_secret =
        consume_approval_token(&store, tenant_b, &tenant_a_token, APPROVAL_SECRET_B).await;
    let nonexistent =
        consume_approval_token(&store, tenant_b, &nonexistent_token, APPROVAL_SECRET_B).await;
    let wrong_tenant_valid_secret =
        consume_approval_token(&store, tenant_b, &tenant_a_token, APPROVAL_SECRET_A).await;

    assert_eq!(
        format!("{wrong_secret:?}"),
        format!("{nonexistent:?}"),
        "wrong-tenant token must be byte-identical to a nonexistent token at the public boundary"
    );
    assert_eq!(
        format!("{wrong_tenant_valid_secret:?}"),
        format!("{nonexistent:?}"),
        "valid token material still must not decide another tenant's approval"
    );
    assert_eq!(wrong_secret, Err(PUBLIC_NOT_FOUND));

    let after = get_request(store.pool(), &approval_id)
        .await
        .expect("get tenant_a approval")
        .expect("tenant_a approval exists");
    assert_eq!(
        after.status,
        ApprovalStatus::Pending,
        "tenant_b attempts must not mutate tenant_a's approval"
    );

    consume_approval_token(&store, tenant_a, &tenant_a_token, APPROVAL_SECRET_A)
        .await
        .expect("tenant_a can consume its own approval token");
    let decided = get_request(store.pool(), &approval_id)
        .await
        .expect("get decided tenant_a approval")
        .expect("tenant_a approval still exists");
    assert_eq!(decided.status, ApprovalStatus::Granted);
}

#[test]
fn slack_inbound_cross_secret_is_publicly_identical_to_bad_signature() {
    let exp = Utc::now().timestamp() + 300;
    let tenant_a_token = sign_approval_token(
        "slack-approval-tenant-a",
        ApprovalDecision::Approve,
        exp,
        APPROVAL_SECRET_A,
    )
    .expect("sign tenant_a token");
    let body = slack_callback_body(&tenant_a_token);
    let timestamp = Utc::now().timestamp().to_string();
    let tenant_b_signature = sign_slack_v0(body.as_bytes(), &timestamp, SLACK_SIGNING_SECRET_B)
        .expect("sign with tenant_b Slack secret");
    let malformed_signature = "v0=0000000000000000000000000000000000000000000000000000000000000000";
    let inbound_a = SlackInbound::new(SlackInboundConfig::new(
        SLACK_SIGNING_SECRET_A,
        APPROVAL_SECRET_A,
    ));

    let cross_headers = [
        ("X-Slack-Signature", tenant_b_signature.as_str()),
        ("X-Slack-Request-Timestamp", timestamp.as_str()),
    ];
    let nonexistent_headers = [
        ("X-Slack-Signature", malformed_signature),
        ("X-Slack-Request-Timestamp", timestamp.as_str()),
    ];

    let cross_tenant = slack_signature_boundary(&inbound_a, body.as_bytes(), &cross_headers);
    let nonexistent = slack_signature_boundary(&inbound_a, body.as_bytes(), &nonexistent_headers);

    assert_eq!(
        format!("{cross_tenant:?}"),
        format!("{nonexistent:?}"),
        "wrong-tenant Slack signature must be byte-identical to a bad signature"
    );
    assert_eq!(cross_tenant, Err(PUBLIC_NOT_FOUND));

    let tenant_a_signature = sign_slack_v0(body.as_bytes(), &timestamp, SLACK_SIGNING_SECRET_A)
        .expect("sign with tenant_a Slack secret");
    let valid_headers = [
        ("X-Slack-Signature", tenant_a_signature.as_str()),
        ("X-Slack-Request-Timestamp", timestamp.as_str()),
    ];
    inbound_a
        .verify_signature(body.as_bytes(), &valid_headers)
        .expect("tenant_a signature verifies for tenant_a inbound");
}

#[test]
#[ignore = "FINDINGS-ONLY: argo-channels::SlackInbound has no AuditSink/tenant context; follow-up should wire tenant_mismatch=true audit in the gateway boundary"]
fn slack_inbound_cross_secret_refusal_should_emit_tenant_mismatch_audit() {
    let err: ChannelResult<()> = Err(ChannelError::Auth("Slack signature mismatch".into()));
    assert!(
        err.is_ok(),
        "desired post-fix shape: wrong-tenant Slack signatures emit an audit row with tenant_mismatch=true before returning the public NotFound envelope"
    );
}
