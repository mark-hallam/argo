//! Integration smoke for `DedupCache` (mission-llm-call-dedup-cache-v2).
//!
//! Asserts the contract that two identical non-streaming requests
//! through a `DedupCache`-wrapped transport collapse into one
//! underlying wire call. Lives in `argo-agent-runner/tests/` because
//! this is where `build_transport` reads the env var; the unit-test
//! suite in `argo-provider::dedup_cache_tests` covers the cache
//! internals (TTL eviction, counters, streaming bypass, per-tenant
//! key separation).

use argo_provider::transport::{ProviderRequest, ProviderResponse, ProviderResponseDelta};
use argo_provider::{DedupCache, Sampling, StopReason, Transport};
use argo_types::{
    ArgoResult, CapabilitySet, ContentBlock, Message, MessageId, MessageRole, SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{self, BoxStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

struct CountingTransport {
    count: Arc<AtomicUsize>,
    caps: CapabilitySet,
}

impl CountingTransport {
    fn new() -> (Self, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let transport = Self {
            count: Arc::clone(&count),
            caps: CapabilitySet(Vec::new()),
        };
        (transport, count)
    }
}

#[async_trait]
impl Transport for CountingTransport {
    fn id(&self) -> &str {
        "smoke-counting"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let n = self.count.fetch_add(1, Ordering::SeqCst);
        Ok(ProviderResponse {
            text: Some(format!("call-{n}")),
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
        Ok(Box::pin(stream::empty()))
    }
}

fn request(session: SessionId, msg_id: MessageId) -> ProviderRequest {
    let created_at = chrono::DateTime::<Utc>::from_timestamp(1_700_000_000, 0)
        .unwrap_or_else(Utc::now);
    ProviderRequest {
        model: "smoke-model".into(),
        messages: vec![Message {
            id: msg_id,
            session_id: session,
            role: MessageRole::User,
            content: vec![ContentBlock::Text { text: "approve this".into() }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            reasoning: None,
            created_at,
            provider_meta: serde_json::Value::Null,
        }],
        tools: Vec::new(),
        system: Some("auto-approval system prompt".into()),
        sampling: Sampling::default(),
    }
}

#[tokio::test]
async fn same_request_twice_collapses_to_one_inner_call() {
    let (inner, count) = CountingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_secs(60));
    let session = SessionId::new();
    let msg_id = MessageId::new();

    let r1 = cache.complete(&request(session, msg_id)).await.unwrap();
    let r2 = cache.complete(&request(session, msg_id)).await.unwrap();

    assert_eq!(count.load(Ordering::SeqCst), 1, "second call must be cached");
    assert_eq!(r1.text, r2.text, "cached response is byte-identical");
    assert_eq!(cache.hits(), 1);
    assert_eq!(cache.misses(), 1);
}

#[tokio::test]
async fn three_distinct_micro_llm_calls_in_one_turn_collapse_to_one() {
    // Simulates the real dedup-win pattern: `auto_approval`,
    // `dispatch`, `safety_judge` all hit the model with the same
    // prompt within a single agent turn.
    let (inner, count) = CountingTransport::new();
    let cache = DedupCache::new(inner, Duration::from_secs(60));
    let session = SessionId::new();
    let msg_id = MessageId::new();
    let req = request(session, msg_id);

    for _ in 0..3 {
        cache.complete(&req).await.unwrap();
    }

    assert_eq!(count.load(Ordering::SeqCst), 1, "all 3 calls collapse to 1");
    assert_eq!(cache.hits(), 2);
    assert_eq!(cache.misses(), 1);
}
