//! Integration test: a mock [`Transport`] that can be toggled to 5xx
//! and a [`BreakerTransport`] wrapping it, observing the
//! Closed → Open → HalfOpen → Closed state machine.

use argo_provider::transport::BreakerTransport;
use argo_provider::{
    Breaker, BreakerConfig, CircuitState, ProviderRequest, ProviderResponse, ProviderResponseDelta,
    Sampling, StopReason, Transport,
};
use argo_types::{ArgoError, ArgoResult, CapabilitySet};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Mock provider with a toggle that swaps successful responses for an
/// HTTP 503 error. The hit counter records every reach-through call.
struct MockProvider {
    id: String,
    capabilities: CapabilitySet,
    fail: Arc<AtomicBool>,
    hits: Arc<AtomicU32>,
}

impl MockProvider {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            capabilities: CapabilitySet::default(),
            fail: Arc::new(AtomicBool::new(false)),
            hits: Arc::new(AtomicU32::new(0)),
        }
    }
}

#[async_trait]
impl Transport for MockProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        self.hits.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(ArgoError::Provider(format!("{} HTTP 503: down", self.id)));
        }
        Ok(ProviderResponse {
            text: Some("ok".into()),
            reasoning: None,
            tool_calls: vec![],
            stop_reason: StopReason::EndTurn,
            tokens_in: Some(1),
            tokens_out: Some(1),
            provider_meta: serde_json::json!({}),
        })
    }

    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        Err(ArgoError::Provider("not implemented in mock".into()))
    }
}

fn req() -> ProviderRequest {
    ProviderRequest {
        model: "mock".into(),
        messages: vec![],
        tools: vec![],
        system: None,
        sampling: Sampling::default(),
    }
}

#[tokio::test]
async fn closed_to_open_after_threshold_then_half_open_then_closed() {
    let mock = MockProvider::new("mock");
    let fail = Arc::clone(&mock.fail);
    let hits = Arc::clone(&mock.hits);
    let breaker = Breaker::new(BreakerConfig {
        failure_threshold: 3,
        cooldown: Duration::from_millis(50),
        cooldown_max: Duration::from_secs(5),
    });
    let wrapped = BreakerTransport::new(mock, breaker.clone());

    fail.store(true, Ordering::SeqCst);
    for _ in 0..3 {
        let _ = wrapped.complete(&req()).await;
    }
    assert_eq!(breaker.state_of("mock"), CircuitState::Open);
    assert_eq!(hits.load(Ordering::SeqCst), 3);

    // Open: subsequent calls do NOT reach the provider.
    let blocked = wrapped.complete(&req()).await.unwrap_err();
    match blocked {
        ArgoError::Provider(msg) => assert!(msg.contains("circuit open")),
        other => panic!("expected Provider, got {other:?}"),
    }
    assert_eq!(hits.load(Ordering::SeqCst), 3, "Open must short-circuit");

    // After cooldown, the next call is admitted as a probe.
    tokio::time::sleep(Duration::from_millis(60)).await;
    fail.store(false, Ordering::SeqCst);
    let resp = wrapped.complete(&req()).await.unwrap();
    assert_eq!(resp.text.as_deref(), Some("ok"));
    assert_eq!(hits.load(Ordering::SeqCst), 4);
    assert_eq!(breaker.state_of("mock"), CircuitState::Closed);
}

#[tokio::test]
async fn http_429_does_not_trip_breaker() {
    struct R429 {
        id: String,
        capabilities: CapabilitySet,
    }
    #[async_trait]
    impl Transport for R429 {
        fn id(&self) -> &str {
            &self.id
        }
        fn capabilities(&self) -> &CapabilitySet {
            &self.capabilities
        }
        async fn complete(&self, _: &ProviderRequest) -> ArgoResult<ProviderResponse> {
            Err(ArgoError::Provider(format!("{} HTTP 429: slow down", self.id)))
        }
        async fn complete_stream(
            &self,
            _: &ProviderRequest,
        ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
            Err(ArgoError::Provider("nope".into()))
        }
    }
    let breaker = Breaker::new(BreakerConfig {
        failure_threshold: 2,
        cooldown: Duration::from_millis(50),
        cooldown_max: Duration::from_secs(5),
    });
    let wrapped = BreakerTransport::new(
        R429 {
            id: "rate".into(),
            capabilities: CapabilitySet::default(),
        },
        breaker.clone(),
    );
    for _ in 0..5 {
        let _ = wrapped.complete(&req()).await;
    }
    assert_eq!(breaker.state_of("rate"), CircuitState::Closed);
}
