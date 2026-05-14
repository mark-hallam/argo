use super::{ObservedBreakerTransport, StateObserver};
use crate::circuit::{Breaker, BreakerConfig, CircuitState};
use crate::transport::{ProviderRequest, ProviderResponse, Transport};
use argo_types::{
    AnomalyAlert, AnomalyDispatch, AnomalyKind, ArgoError, ArgoResult, CapabilitySet,
};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Minimal Transport used in tests — never invoked, just needed so
/// `ObservedBreakerTransport::new` has an `inner.id()` to read.
struct ProbeTransport {
    id: String,
    caps: CapabilitySet,
}

#[async_trait]
impl Transport for ProbeTransport {
    fn id(&self) -> &str {
        &self.id
    }
    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }
    async fn complete(
        &self,
        _req: &crate::transport::ProviderRequest,
    ) -> ArgoResult<crate::transport::ProviderResponse> {
        unreachable!("ProbeTransport.complete is never invoked in unit tests")
    }
    async fn complete_stream(
        &self,
        _req: &crate::transport::ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<crate::transport::ProviderResponseDelta>>> {
        unreachable!("ProbeTransport.complete_stream is never invoked in unit tests")
    }
}

struct FailingTransport {
    id: String,
    caps: CapabilitySet,
}

#[async_trait]
impl Transport for FailingTransport {
    fn id(&self) -> &str {
        &self.id
    }
    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }
    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        Err(ArgoError::Provider("test-provider HTTP 500: boom".into()))
    }
    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<crate::transport::ProviderResponseDelta>>> {
        Err(ArgoError::Provider("test-provider HTTP 500: boom".into()))
    }
}

struct CountingDispatch {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl AnomalyDispatch for CountingDispatch {
    async fn dispatch(&self, alert: AnomalyAlert) -> ArgoResult<()> {
        assert_eq!(alert.kind, AnomalyKind::ProviderStorm);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn dummy_request() -> ProviderRequest {
    ProviderRequest {
        model: "test".into(),
        messages: Vec::new(),
        tools: Vec::new(),
        system: None,
        sampling: Default::default(),
    }
}

#[test]
fn closed_to_open_emits_once_and_returns_transition() {
    let obs = StateObserver::new("anthropic");
    // First observe of Closed matches the initial cached value → no transition.
    assert_eq!(obs.observe(CircuitState::Closed), None);
    // Then a real transition.
    assert_eq!(
        obs.observe(CircuitState::Open),
        Some((CircuitState::Closed, CircuitState::Open))
    );
    // Subsequent observes of the same state are silent (dedup).
    assert_eq!(obs.observe(CircuitState::Open), None);
    assert_eq!(obs.observe(CircuitState::Open), None);
    assert_eq!(obs.last_state(), CircuitState::Open);
}

#[test]
fn stable_state_never_returns_transition() {
    let obs = StateObserver::new("openai");
    for _ in 0..10 {
        assert_eq!(obs.observe(CircuitState::Closed), None);
    }
    assert_eq!(obs.last_state(), CircuitState::Closed);
}

#[test]
fn half_open_to_closed_returns_transition() {
    let obs = StateObserver::new("bedrock");
    // Walk through the recovery path: Closed → Open → HalfOpen → Closed.
    assert_eq!(
        obs.observe(CircuitState::Open),
        Some((CircuitState::Closed, CircuitState::Open))
    );
    assert_eq!(
        obs.observe(CircuitState::HalfOpen),
        Some((CircuitState::Open, CircuitState::HalfOpen))
    );
    assert_eq!(
        obs.observe(CircuitState::Closed),
        Some((CircuitState::HalfOpen, CircuitState::Closed))
    );
    // Same state again → silent.
    assert_eq!(obs.observe(CircuitState::Closed), None);
}

#[test]
fn observed_transport_caches_provider_id_from_inner() {
    let probe = ProbeTransport {
        id: "openai-compat-test".into(),
        caps: CapabilitySet::default(),
    };
    let breaker = Breaker::new(BreakerConfig::default());
    let observed = ObservedBreakerTransport::new(probe, breaker);
    assert_eq!(observed.id(), "openai-compat-test");
    assert_eq!(observed.observer().provider_id(), "openai-compat-test");
    // Freshly constructed breaker reports Closed.
    assert_eq!(
        observed.breaker().state_of("openai-compat-test"),
        CircuitState::Closed
    );
}

#[test]
fn observed_transport_tracks_breaker_state_after_failures() {
    let probe = ProbeTransport {
        id: "anthropic-test".into(),
        caps: CapabilitySet::default(),
    };
    // failure_threshold=1 trips on a single failure.
    let breaker = Breaker::new(BreakerConfig {
        failure_threshold: 1,
        cooldown: Duration::from_millis(1),
        cooldown_max: Duration::from_millis(1),
    });
    let observed = ObservedBreakerTransport::new(probe, breaker);
    // Initially Closed; first observe matches the cache → silent.
    assert_eq!(
        observed.breaker().state_of("anthropic-test"),
        CircuitState::Closed
    );
    // Drive a failure through the breaker directly — that's
    // exactly what `BreakerTransport::complete` would do on a 5xx.
    let permit = observed.breaker().try_acquire("anthropic-test").unwrap();
    permit.record_failure();
    assert_eq!(
        observed.breaker().state_of("anthropic-test"),
        CircuitState::Open
    );
    // The observer has not yet sampled the new state, so
    // last_state still reads Closed (the initial cache value).
    assert_eq!(observed.observer().last_state(), CircuitState::Closed);
    // Now sample via the observer — should report the transition.
    let cur = observed.breaker().state_of("anthropic-test");
    assert_eq!(
        observed.observer().observe(cur),
        Some((CircuitState::Closed, CircuitState::Open))
    );
}

#[tokio::test]
async fn observed_transport_dispatches_provider_storm_on_open() {
    let calls = Arc::new(AtomicUsize::new(0));
    let probe = FailingTransport {
        id: "storm-test".into(),
        caps: CapabilitySet::default(),
    };
    let breaker = Breaker::new(BreakerConfig {
        failure_threshold: 1,
        cooldown: Duration::from_millis(1),
        cooldown_max: Duration::from_millis(1),
    });
    let dispatch = Arc::new(CountingDispatch {
        calls: Arc::clone(&calls),
    });
    let observed = ObservedBreakerTransport::with_anomaly_dispatch(probe, breaker, dispatch);
    let err = observed
        .complete(&dummy_request())
        .await
        .expect_err("failing provider");
    assert!(format!("{err}").contains("HTTP 500"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(observed.observer().last_state(), CircuitState::Open);
}
