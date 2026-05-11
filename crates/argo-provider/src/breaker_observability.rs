//! Per-provider breaker state-transition observability.
//!
//! [`StateObserver`] caches the last-seen [`CircuitState`] in an
//! `AtomicU8` and emits exactly one `tracing::warn!` per transition
//! (Closed→Open, Open→HalfOpen, HalfOpen→{Open,Closed}, etc.).
//!
//! [`ObservedBreakerTransport`] composes [`BreakerTransport`] (the
//! gating wrapper from `transport.rs`) with a [`StateObserver`] so
//! every provider call gets gated AND a state-transition warning
//! fires once on every change. Each provider client gets its own
//! observer + breaker — they are not shared between providers.

use crate::circuit::{Breaker, CircuitState};
use crate::transport::{
    BreakerTransport, ProviderRequest, ProviderResponse, ProviderResponseDelta, Transport,
};
use argo_types::{ArgoResult, CapabilitySet};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::atomic::{AtomicU8, Ordering};

const CLOSED_U8: u8 = 0;
const OPEN_U8: u8 = 1;
const HALF_OPEN_U8: u8 = 2;

fn state_to_u8(s: CircuitState) -> u8 {
    match s {
        CircuitState::Closed => CLOSED_U8,
        CircuitState::Open => OPEN_U8,
        CircuitState::HalfOpen => HALF_OPEN_U8,
    }
}

fn u8_to_state(v: u8) -> CircuitState {
    match v {
        OPEN_U8 => CircuitState::Open,
        HALF_OPEN_U8 => CircuitState::HalfOpen,
        _ => CircuitState::Closed,
    }
}

/// Last-state cache for one provider's breaker.
///
/// Use [`Self::observe`] each time you sample the breaker's current
/// [`CircuitState`]. Returns `Some((prev, curr))` and emits a single
/// `tracing::warn!` on the first observe-call that sees a state
/// change; subsequent observes that see the same state return
/// `None` and emit nothing. Cheap atomic swap → no lock contention
/// across concurrent provider calls.
///
/// ```
/// use argo_provider::breaker_observability::StateObserver;
/// use argo_provider::CircuitState;
/// let obs = StateObserver::new("anthropic");
/// assert_eq!(obs.observe(CircuitState::Closed), None);
/// assert_eq!(
///     obs.observe(CircuitState::Open),
///     Some((CircuitState::Closed, CircuitState::Open))
/// );
/// assert_eq!(obs.observe(CircuitState::Open), None);
/// ```
#[derive(Debug)]
pub struct StateObserver {
    provider_id: String,
    last: AtomicU8,
}

impl StateObserver {
    /// Build an observer initialised to `Closed` (the breaker's
    /// own default-on-create state).
    #[must_use]
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            last: AtomicU8::new(CLOSED_U8),
        }
    }

    /// Provider id this observer reports against.
    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Compare-and-swap the cached state. Returns `Some((prev, curr))`
    /// (and emits `tracing::warn!`) on transition; `None` otherwise.
    pub fn observe(&self, current: CircuitState) -> Option<(CircuitState, CircuitState)> {
        let cur_u8 = state_to_u8(current);
        let prev_u8 = self.last.swap(cur_u8, Ordering::SeqCst);
        if prev_u8 == cur_u8 {
            return None;
        }
        let prev = u8_to_state(prev_u8);
        tracing::warn!(
            provider = %self.provider_id,
            previous = ?prev,
            current = ?current,
            "circuit_breaker_state_transition"
        );
        Some((prev, current))
    }

    /// Snapshot of the cached state without mutating it.
    #[must_use]
    pub fn last_state(&self) -> CircuitState {
        u8_to_state(self.last.load(Ordering::SeqCst))
    }
}

/// Transport wrapper that gates calls through a [`BreakerTransport`]
/// AND emits a `tracing::warn!` exactly once per breaker state
/// transition.
///
/// Each provider client owns one of these — the inner [`Breaker`] is
/// per-provider, not shared across providers (one provider's outage
/// must not trip another's circuit).
///
/// ```
/// # async fn example<T: argo_provider::Transport>(inner: T) {
/// use argo_provider::breaker_observability::ObservedBreakerTransport;
/// use argo_provider::{Breaker, BreakerConfig};
/// let breaker = Breaker::new(BreakerConfig::default());
/// let _wrapped = ObservedBreakerTransport::new(inner, breaker);
/// # }
/// ```
pub struct ObservedBreakerTransport<T: Transport> {
    inner: BreakerTransport<T>,
    observer: StateObserver,
}

impl<T: Transport> ObservedBreakerTransport<T> {
    /// Wrap `inner` with a breaker + observer in one shot. The
    /// observer keys off `inner.id()` so warnings name the provider.
    pub fn new(inner: T, breaker: Breaker) -> Self {
        let observer = StateObserver::new(inner.id().to_string());
        Self {
            inner: BreakerTransport::new(inner, breaker),
            observer,
        }
    }

    /// Borrowed breaker — the CLI inspector uses this to read state.
    pub fn breaker(&self) -> &Breaker {
        self.inner.breaker()
    }

    /// Borrowed inner concrete transport (test access).
    pub fn inner_transport(&self) -> &T {
        self.inner.inner()
    }

    /// Borrowed observer (test access).
    pub fn observer(&self) -> &StateObserver {
        &self.observer
    }

    /// Sample the current state and run it through the observer.
    fn observe_current(&self) {
        let cur = self.inner.breaker().state_of(self.observer.provider_id());
        let _ = self.observer.observe(cur);
    }
}

#[async_trait]
impl<T: Transport> Transport for ObservedBreakerTransport<T> {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn capabilities(&self) -> &CapabilitySet {
        self.inner.capabilities()
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let result = self.inner.complete(req).await;
        self.observe_current();
        result
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        let result = self.inner.complete_stream(req).await;
        self.observe_current();
        result
    }
}

#[cfg(test)]
#[path = "breaker_observability_tests.rs"]
mod tests;
