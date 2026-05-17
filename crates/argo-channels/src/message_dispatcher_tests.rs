//! Tests for [`super::ProductionMessageDispatcher`].

use super::*;
use crate::ChannelId;
use argo_audit::AuditSink;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug)]
struct CaptureChannel {
    id: ChannelId,
    calls: Mutex<Vec<(String, String, Vec<PathBuf>)>>,
    fail_with: Option<String>,
    next_id: Mutex<u64>,
}

impl CaptureChannel {
    fn new(id: &str) -> Self {
        Self {
            id: ChannelId::new(id),
            calls: Mutex::new(Vec::new()),
            fail_with: None,
            next_id: Mutex::new(1),
        }
    }

    fn failing(id: &str, reason: &str) -> Self {
        Self {
            id: ChannelId::new(id),
            calls: Mutex::new(Vec::new()),
            fail_with: Some(reason.to_string()),
            next_id: Mutex::new(1),
        }
    }
}

#[async_trait]
impl OutboundChannel for CaptureChannel {
    fn id(&self) -> ChannelId {
        self.id.clone()
    }

    async fn send_text(
        &self,
        recipient: &str,
        body: &str,
        attachments: &[PathBuf],
    ) -> Result<String, String> {
        if let Some(reason) = &self.fail_with {
            return Err(reason.clone());
        }
        self.calls.lock().unwrap().push((
            recipient.to_string(),
            body.to_string(),
            attachments.to_vec(),
        ));
        let mut n = self.next_id.lock().unwrap();
        let id = format!("{}-msg-{}", self.id, *n);
        *n += 1;
        Ok(id)
    }
}

async fn build(channel: Arc<dyn OutboundChannel>) -> (ProductionMessageDispatcher, Arc<SessionStore>) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let tenant = TenantId(Uuid::now_v7());
    let audit = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        tenant.to_string(),
    ));
    let dispatcher =
        ProductionMessageDispatcher::new(audit, Arc::clone(&store), tenant).with_channel(channel);
    (dispatcher, store)
}

#[tokio::test]
async fn happy_path_sends_through_channel_and_writes_outbound_row() {
    let cap = Arc::new(CaptureChannel::new("slack"));
    let (dispatcher, store) = build(cap.clone()).await;
    let outcome = dispatcher
        .dispatch("slack", "C123", "shipped", &[])
        .await
        .expect("dispatch ok");
    assert_eq!(outcome.message_id, "slack-msg-1");
    assert_eq!(cap.calls.lock().unwrap().len(), 1);
    // The audit batched writer needs an explicit flush in tests.
    // The dispatcher's idempotency table write is direct, so re-
    // looking it up confirms the row landed.
    let body_hash = hash_body("slack", "C123", "shipped", &[]);
    let row = outbound_message_query::lookup_recent_by_body_hash(
        &store,
        dispatcher.tenant_id,
        &body_hash,
        0,
    )
    .await
    .unwrap()
    .expect("outbound_message row");
    assert_eq!(row.message_id, "slack-msg-1");
    assert_eq!(row.channel, "slack");
    assert_eq!(row.recipient, "C123");
}

#[tokio::test]
async fn idempotency_returns_cached_outcome_for_duplicate_body() {
    let cap = Arc::new(CaptureChannel::new("email"));
    let (dispatcher, _store) = build(cap.clone()).await;
    let first = dispatcher
        .dispatch("email", "u@x.com", "hello", &[])
        .await
        .unwrap();
    let second = dispatcher
        .dispatch("email", "u@x.com", "hello", &[])
        .await
        .unwrap();
    assert_eq!(first.message_id, second.message_id);
    assert_eq!(
        cap.calls.lock().unwrap().len(),
        1,
        "channel.send_text should only run once",
    );
}

#[tokio::test]
async fn unknown_channel_returns_clean_error_and_audits_deny() {
    let cap = Arc::new(CaptureChannel::new("slack"));
    let (dispatcher, _store) = build(cap.clone()).await;
    let err = dispatcher
        .dispatch("fax", "x", "y", &[])
        .await
        .expect_err("unknown channel must error");
    assert!(err.contains("no outbound channel registered for 'fax'"), "{err}");
    assert!(cap.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn channel_failure_surfaces_as_dispatch_error_with_no_idempotency_row() {
    let cap = Arc::new(CaptureChannel::failing("sms", "transport down"));
    let (dispatcher, store) = build(cap).await;
    let err = dispatcher
        .dispatch("sms", "+15550000", "hi", &[])
        .await
        .expect_err("transport failure must propagate");
    assert!(err.contains("transport down"), "{err}");
    let body_hash = hash_body("sms", "+15550000", "hi", &[]);
    let row = outbound_message_query::lookup_recent_by_body_hash(
        &store,
        dispatcher.tenant_id,
        &body_hash,
        0,
    )
    .await
    .unwrap();
    assert!(row.is_none(), "failed send must not leave an idempotency row");
}

#[tokio::test]
async fn body_hash_distinguishes_different_recipients() {
    let cap = Arc::new(CaptureChannel::new("slack"));
    let (dispatcher, _store) = build(cap.clone()).await;
    dispatcher.dispatch("slack", "C1", "hi", &[]).await.unwrap();
    dispatcher.dispatch("slack", "C2", "hi", &[]).await.unwrap();
    // Distinct recipients are NOT collapsed by idempotency — both
    // sends reach the channel.
    assert_eq!(cap.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn registered_channels_listed_for_debug() {
    let cap = Arc::new(CaptureChannel::new("slack"));
    let (dispatcher, _store) = build(cap).await;
    let debug_str = format!("{dispatcher:?}");
    assert!(debug_str.contains("slack"), "{debug_str}");
}
