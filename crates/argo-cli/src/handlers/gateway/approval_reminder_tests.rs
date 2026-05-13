use super::*;
use argo_identity::NotifyChannelRef;
use argo_store::approvals::{
    create_request, get_request, mark_escalation_sent_at, DefaultDecision, NewApprovalRequest,
};
use argo_types::TenantId;
use std::sync::Mutex;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn seed(store: &SessionStore, deadline_at: i64, action_byte: u8) -> String {
    create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: vec![action_byte],
            tenant_id: TenantId::new(),
            requested_by: "operator",
            deadline_at,
            default_decision: DefaultDecision::Deny,
        },
    )
    .await
    .expect("create approval")
}

fn config() -> NotifyConfig {
    NotifyConfig {
        channels: vec![NotifyChannelRef::new("webinbox", "operator")],
        notify_channels: Default::default(),
    }
}

#[derive(Default)]
struct RecordingSender {
    sent: Mutex<Vec<Notification>>,
    fail: bool,
}

#[async_trait::async_trait]
impl NotifySender for RecordingSender {
    async fn send_via(
        &self,
        _ref_: &NotifyChannelRef,
        notification: &Notification,
    ) -> Result<(), String> {
        if self.fail {
            return Err("simulated send failure".into());
        }
        self.sent.lock().unwrap().push(notification.clone());
        Ok(())
    }
}

#[tokio::test]
async fn scan_finds_near_deadline_and_stamps_after_send() {
    let store = fresh_store().await;
    let now = Utc.timestamp_micros(1_700_000_000_000_000).unwrap();
    let id = seed(&store, now.timestamp_micros() + 60_000_000, 1).await;
    let sender = RecordingSender::default();

    let stamped = sweep_once_with_config(&store, &config(), now, Duration::from_secs(120), &sender)
        .await
        .unwrap();

    assert_eq!(stamped, 1);
    assert_eq!(sender.sent.lock().unwrap().len(), 1);
    let row = get_request(store.pool(), &id).await.unwrap().unwrap();
    assert!(row.escalation_sent_at.is_some());
}

#[tokio::test]
async fn scan_skips_already_escalated() {
    let store = fresh_store().await;
    let now = Utc.timestamp_micros(1_700_000_000_000_000).unwrap();
    let id = seed(&store, now.timestamp_micros() + 60_000_000, 2).await;
    mark_escalation_sent_at(store.pool(), &id, now.timestamp_micros())
        .await
        .unwrap();
    let sender = RecordingSender::default();

    let stamped = sweep_once_with_config(&store, &config(), now, Duration::from_secs(120), &sender)
        .await
        .unwrap();

    assert_eq!(stamped, 0);
    assert!(sender.sent.lock().unwrap().is_empty());
}

#[tokio::test]
async fn scan_skips_future_beyond_window() {
    let store = fresh_store().await;
    let now = Utc.timestamp_micros(1_700_000_000_000_000).unwrap();
    seed(&store, now.timestamp_micros() + 3_600_000_000, 3).await;
    let sender = RecordingSender::default();

    let stamped = sweep_once_with_config(&store, &config(), now, Duration::from_secs(120), &sender)
        .await
        .unwrap();

    assert_eq!(stamped, 0);
    assert!(sender.sent.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failed_dispatch_does_not_stamp() {
    let store = fresh_store().await;
    let now = Utc.timestamp_micros(1_700_000_000_000_000).unwrap();
    let id = seed(&store, now.timestamp_micros() + 60_000_000, 4).await;
    let sender = RecordingSender {
        sent: Mutex::new(Vec::new()),
        fail: true,
    };

    let stamped = sweep_once_with_config(&store, &config(), now, Duration::from_secs(120), &sender)
        .await
        .unwrap();

    assert_eq!(stamped, 0);
    let row = get_request(store.pool(), &id).await.unwrap().unwrap();
    assert!(row.escalation_sent_at.is_none());
}
