//! Integration tests for [`crate::dispatcher`]. Each test opens an
//! in-memory `SessionStore`, seeds `audit_record` rows, then drives
//! the dispatcher with a counting / failing mock sink to verify
//! cursor + retry semantics.
use super::*;
use crate::sink::{AuditEvent, SiemSink, SinkError};
use argo_store::SessionStore;
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Records every batch shipped + lets a test inject a sequence of
/// scripted responses. Each entry in `script` is consumed in order;
/// running off the end of the script yields `Ok(())` so happy-path
/// tests don't need to size the script perfectly.
struct MockSink {
    backend: &'static str,
    endpoint: String,
    max_batch: usize,
    shipped: Mutex<Vec<Vec<AuditEvent>>>,
    script: Mutex<Vec<Result<(), SinkError>>>,
    calls: AtomicU64,
}

impl MockSink {
    fn new(backend: &'static str) -> Arc<Self> {
        Arc::new(Self {
            backend,
            endpoint: "https://mock".to_string(),
            max_batch: 10,
            shipped: Mutex::new(Vec::new()),
            script: Mutex::new(Vec::new()),
            calls: AtomicU64::new(0),
        })
    }

    fn with_script(self: Arc<Self>, script: Vec<Result<(), SinkError>>) -> Arc<Self> {
        *self.script.lock().unwrap() = script.into_iter().rev().collect();
        self
    }

    fn calls(&self) -> u64 {
        self.calls.load(Ordering::SeqCst)
    }

    fn shipped(&self) -> Vec<Vec<AuditEvent>> {
        self.shipped.lock().unwrap().clone()
    }
}

#[async_trait]
impl SiemSink for MockSink {
    fn backend(&self) -> &'static str {
        self.backend
    }
    fn endpoint(&self) -> &str {
        &self.endpoint
    }
    fn max_batch(&self) -> usize {
        self.max_batch
    }
    async fn ship(&self, events: Vec<AuditEvent>) -> Result<(), SinkError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let outcome = self.script.lock().unwrap().pop().unwrap_or(Ok(()));
        if outcome.is_ok() {
            self.shipped.lock().unwrap().push(events);
        }
        outcome
    }
}

async fn open_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("open store");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

async fn insert_audit(store: &SessionStore, kind: &str, t: i64) {
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(&t.to_be_bytes());
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at) \
         VALUES (?, NULL, ?, '{\"k\":1}', ?, ?, ?)",
    )
    .bind(id.as_slice())
    .bind(kind)
    .bind(vec![0u8; 32])
    .bind(vec![1u8; 32])
    .bind(t)
    .execute(store.pool())
    .await
    .expect("audit insert");
}

#[tokio::test]
async fn sweep_returns_none_when_no_rows_pending() {
    let store = open_store().await;
    let sink = MockSink::new("mock");
    let d = SiemDispatcher::new(store, sink.clone(), SiemDispatcherConfig::default());
    d.ensure_schema().await.unwrap();
    assert_eq!(d.sweep_once().await.unwrap(), None);
    assert_eq!(sink.calls(), 0);
}

#[tokio::test]
async fn sweep_ships_pending_rows_and_advances_cursor() {
    let store = open_store().await;
    for t in 1..=3 {
        insert_audit(&store, "k", t).await;
    }
    let sink = MockSink::new("mock");
    let d = SiemDispatcher::new(store, sink.clone(), SiemDispatcherConfig::default());
    d.ensure_schema().await.unwrap();
    let high = d.sweep_once().await.unwrap();
    assert_eq!(high, Some(3));
    assert_eq!(sink.calls(), 1, "single ship call for 3 rows under batch cap");
    let batches = sink.shipped();
    assert_eq!(batches[0].len(), 3);
    // Second sweep should be a no-op now that the cursor caught up.
    assert_eq!(d.sweep_once().await.unwrap(), None);
}

#[tokio::test]
async fn cursor_persists_across_dispatcher_instances() {
    let store = open_store().await;
    insert_audit(&store, "a", 1).await;
    insert_audit(&store, "a", 2).await;
    {
        let sink = MockSink::new("mock");
        let d = SiemDispatcher::new(
            Arc::clone(&store),
            sink,
            SiemDispatcherConfig::default(),
        );
        d.ensure_schema().await.unwrap();
        d.sweep_once().await.unwrap();
    }
    // Simulate restart — fresh dispatcher, same store + sink.
    insert_audit(&store, "a", 3).await;
    let sink2 = MockSink::new("mock");
    let d2 = SiemDispatcher::new(
        Arc::clone(&store),
        sink2.clone(),
        SiemDispatcherConfig::default(),
    );
    d2.ensure_schema().await.unwrap();
    let high = d2.sweep_once().await.unwrap();
    assert_eq!(high, Some(3));
    // Only one new row was shipped post-restart, not all three.
    let batches = sink2.shipped();
    assert_eq!(batches[0].len(), 1, "resumes after persisted cursor — no replay");
    assert_eq!(batches[0][0].seq, 3);
}

#[tokio::test]
async fn transient_failure_retries_then_succeeds() {
    let store = open_store().await;
    insert_audit(&store, "k", 1).await;
    let sink = MockSink::new("mock").with_script(vec![
        Err(SinkError::Transient("first".into())),
        Err(SinkError::Transient("second".into())),
        Ok(()),
    ]);
    let d = SiemDispatcher::new(store, sink.clone(), SiemDispatcherConfig::default());
    d.ensure_schema().await.unwrap();
    let high = d.sweep_once().await.unwrap();
    assert_eq!(high, Some(1));
    assert_eq!(sink.calls(), 3, "two transient retries + one success");
}

#[tokio::test]
async fn permanent_failure_advances_cursor_past_poison_batch() {
    let store = open_store().await;
    insert_audit(&store, "k", 1).await;
    insert_audit(&store, "k", 2).await;
    let sink = MockSink::new("mock")
        .with_script(vec![Err(SinkError::Permanent("bad shape".into()))]);
    let d = SiemDispatcher::new(store, sink.clone(), SiemDispatcherConfig::default());
    d.ensure_schema().await.unwrap();
    let high = d.sweep_once().await.unwrap();
    assert_eq!(high, Some(2), "cursor advances past poison batch");
    // Cursor should now sit at 2 — no replay on the next sweep.
    assert_eq!(d.sweep_once().await.unwrap(), None);
}

#[tokio::test]
async fn transient_retries_exhausted_becomes_permanent() {
    let store = open_store().await;
    insert_audit(&store, "k", 1).await;
    let sink = MockSink::new("mock").with_script(vec![
        Err(SinkError::Transient("net".into())),
        Err(SinkError::Transient("net".into())),
        Err(SinkError::Transient("net".into())),
        Err(SinkError::Transient("net".into())),
        Err(SinkError::Transient("net".into())),
    ]);
    let d = SiemDispatcher::new(store, sink.clone(), SiemDispatcherConfig::default());
    d.ensure_schema().await.unwrap();
    // The exhausted-transient case is converted to Permanent inside
    // ship_with_retries; sweep_once observes it and advances the
    // cursor rather than surfacing an error to the caller.
    let high = d.sweep_once().await.unwrap();
    assert_eq!(high, Some(1));
}

#[tokio::test]
async fn batch_cap_caps_load_size() {
    let store = open_store().await;
    for t in 1..=25 {
        insert_audit(&store, "k", t).await;
    }
    let sink = MockSink::new("mock");
    let d = SiemDispatcher::new(store, sink.clone(), SiemDispatcherConfig::default());
    d.ensure_schema().await.unwrap();
    let first = d.sweep_once().await.unwrap();
    assert_eq!(first, Some(10), "first sweep ships exactly max_batch (10) rows");
    let second = d.sweep_once().await.unwrap();
    assert_eq!(second, Some(20));
    let third = d.sweep_once().await.unwrap();
    assert_eq!(third, Some(25));
}

