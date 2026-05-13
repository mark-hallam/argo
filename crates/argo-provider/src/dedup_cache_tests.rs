//! Unit tests for [`super::DedupCache`].
use super::*;
use crate::transport::{Sampling, StopReason};
use argo_types::{
    ArgoResult, CapabilitySet, ContentBlock, Message, MessageId, MessageRole, SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{self, BoxStream};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

/// Recording fake; per-method counters.
struct RecordingTransport {
    count: Arc<AtomicUsize>,
    stream_count: Arc<AtomicUsize>,
    caps: CapabilitySet,
}

impl RecordingTransport {
    fn new() -> (Self, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let stream_count = Arc::new(AtomicUsize::new(0));
        let t = Self {
            count: Arc::clone(&count),
            stream_count: Arc::clone(&stream_count),
            caps: CapabilitySet(Vec::new()),
        };
        (t, count, stream_count)
    }
}

#[async_trait]
impl Transport for RecordingTransport {
    fn id(&self) -> &str {
        "recording-fake"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let n = self.count.fetch_add(1, Ordering::SeqCst);
        Ok(ProviderResponse {
            text: Some(format!("ok-{n}")),
            reasoning: None,
            tool_calls: Vec::new(),
            stop_reason: StopReason::EndTurn,
            tokens_in: None,
            tokens_out: None,
            provider_meta: serde_json::json!({}),
        })
    }

    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        self.stream_count.fetch_add(1, Ordering::SeqCst);
        Ok(Box::pin(stream::empty()))
    }
}

/// Pinned-clock helper: `MessageId` + `created_at` are in the
/// fingerprint, so tests that need a byte-identical second request
/// share both with the first.
fn mk_req(session: SessionId, msg_id: MessageId, text: &str) -> ProviderRequest {
    let created_at = chrono::DateTime::<Utc>::from_timestamp(1_700_000_000, 0)
        .unwrap_or_else(Utc::now);
    ProviderRequest {
        model: "test-model".into(),
        messages: vec![Message {
            id: msg_id,
            session_id: session,
            role: MessageRole::User,
            content: vec![ContentBlock::Text { text: text.into() }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            reasoning: None,
            created_at,
            provider_meta: serde_json::Value::Null,
        }],
        tools: Vec::new(),
        system: None,
        sampling: Sampling::default(),
    }
}

#[tokio::test]
async fn hit_within_ttl_returns_cached_and_skips_inner() {
    let (inner, count, _) = RecordingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_secs(60));
    let session = SessionId::new();
    let msg_id = MessageId::new();
    let req1 = mk_req(session, msg_id, "hi");
    let req2 = mk_req(session, msg_id, "hi");

    let r1 = cache.complete(&req1).await.expect("first call");
    let r2 = cache.complete(&req2).await.expect("second call (cached)");

    assert_eq!(count.load(Ordering::SeqCst), 1, "inner called exactly once");
    assert_eq!(r1.text, r2.text, "cached response returned verbatim");
    assert_eq!(cache.hits(), 1);
    assert_eq!(cache.misses(), 1);
}

#[tokio::test]
async fn miss_outside_ttl_falls_through_to_inner() {
    let (inner, count, _) = RecordingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_millis(40));
    let session = SessionId::new();
    let msg_id = MessageId::new();
    let req = mk_req(session, msg_id, "hi");

    cache.complete(&req).await.expect("first call");
    tokio::time::sleep(Duration::from_millis(80)).await;
    cache.complete(&req).await.expect("second call (stale)");

    assert_eq!(count.load(Ordering::SeqCst), 2, "TTL expiry → 2nd wire call");
    assert_eq!(cache.misses(), 2);
    assert_eq!(cache.hits(), 0);
}

#[tokio::test]
async fn evict_stale_drops_entries_older_than_ttl() {
    let (inner, _count, _) = RecordingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_millis(30));
    let req = mk_req(SessionId::new(), MessageId::new(), "hi");
    cache.complete(&req).await.expect("seed");
    assert_eq!(cache.len(), 1);
    tokio::time::sleep(Duration::from_millis(60)).await;
    cache.evict_stale(Instant::now());
    assert!(cache.is_empty(), "stale entry evicted");
}

#[tokio::test]
async fn streaming_calls_bypass_cache_entirely() {
    let (inner, complete_count, stream_count) = RecordingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_secs(60));
    let session = SessionId::new();
    let req = mk_req(session, MessageId::new(), "hi");

    // Two streaming calls — both must hit inner.
    let _ = cache.complete_stream(&req).await.expect("stream 1");
    let _ = cache.complete_stream(&req).await.expect("stream 2");

    assert_eq!(stream_count.load(Ordering::SeqCst), 2, "no caching on streams");
    assert_eq!(complete_count.load(Ordering::SeqCst), 0);
    assert_eq!(cache.hits(), 0, "streams don't touch the hit counter");
    assert_eq!(cache.misses(), 0);
}

#[tokio::test]
async fn per_tenant_isolation_via_session_id_in_messages() {
    let (inner, count, _) = RecordingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_secs(60));
    // Same body text, but different `session_id` (UUID per tenant
    // session). Fingerprint must NOT collide.
    let req_tenant_a = mk_req(SessionId::new(), MessageId::new(), "same prompt");
    let req_tenant_b = mk_req(SessionId::new(), MessageId::new(), "same prompt");

    cache.complete(&req_tenant_a).await.expect("tenant A");
    cache.complete(&req_tenant_b).await.expect("tenant B");

    assert_eq!(count.load(Ordering::SeqCst), 2, "different sessions → no shared cache");
    assert_eq!(cache.hits(), 0);
    assert_eq!(cache.misses(), 2);
}

#[tokio::test]
async fn hit_counter_tracks_exact_hit_count() {
    let (inner, _count, _) = RecordingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_secs(60));
    let session = SessionId::new();
    let msg_id = MessageId::new();
    let req = mk_req(session, msg_id, "x");

    cache.complete(&req).await.unwrap(); // miss
    for _ in 0..3 {
        cache.complete(&req).await.unwrap(); // 3 hits
    }
    assert_eq!(cache.hits(), 3);
    assert_eq!(cache.misses(), 1);
}

#[tokio::test]
async fn miss_counter_tracks_distinct_requests() {
    let (inner, _count, _) = RecordingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_secs(60));

    // Four distinct sessions → four misses, zero hits.
    for _ in 0..4 {
        let req = mk_req(SessionId::new(), MessageId::new(), "x");
        cache.complete(&req).await.unwrap();
    }
    assert_eq!(cache.misses(), 4);
    assert_eq!(cache.hits(), 0);
}
