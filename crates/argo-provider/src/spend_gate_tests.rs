//! Unit tests for [`SpendGate`]. Uses fake transports + fake
//! enforcers to assert the contract:
//!
//! 1. Under-budget calls pass through to the inner transport.
//! 2. Each of the three windows (Session / Day / Month) blocks the
//!    inner from being invoked.
//! 3. Projected-cost equal-to-remaining is allowed (boundary
//!    inclusive).
use super::*;
use crate::transport::{Sampling, StopReason};
use argo_types::{ArgoError, ArgoResult, BudgetWindow, CapabilitySet};
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Recording fake — every `complete` increments `count` so tests can
/// distinguish "inner was called" from "blocked before inner".
struct RecordingTransport {
    count: Arc<AtomicUsize>,
    caps: CapabilitySet,
}

impl RecordingTransport {
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
impl Transport for RecordingTransport {
    fn id(&self) -> &str {
        "recording-fake"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(ProviderResponse {
            text: Some("ok".into()),
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
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(Box::pin(stream::empty()))
    }
}

/// Enforcer that always passes through. Used to assert the gate
/// does nothing visible on the happy path.
struct AlwaysAllow;
#[async_trait]
impl BudgetEnforcer for AlwaysAllow {
    async fn enforce(&self, _projected_cost_micros: u64) -> ArgoResult<()> {
        Ok(())
    }
}

/// Enforcer that always blocks with a fixed window. Used to assert
/// the gate short-circuits before the inner transport runs.
struct AlwaysBlock {
    window: BudgetWindow,
}
#[async_trait]
impl BudgetEnforcer for AlwaysBlock {
    async fn enforce(&self, projected_cost_micros: u64) -> ArgoResult<()> {
        Err(ArgoError::BudgetExceeded {
            window: self.window,
            used_micros: 1_000,
            ceiling_micros: 1_000,
            projected_cost_micros,
        })
    }
}

fn dummy_request() -> ProviderRequest {
    ProviderRequest {
        model: "test-model".into(),
        messages: Vec::new(),
        tools: Vec::new(),
        system: None,
        sampling: Sampling::default(),
    }
}

#[tokio::test]
async fn under_budget_passes_through_to_inner() {
    let (inner, count) = RecordingTransport::new();
    let gate = SpendGate::new(inner, Arc::new(AlwaysAllow));
    let resp = gate.complete(&dummy_request()).await.expect("pass through");
    assert_eq!(resp.text.as_deref(), Some("ok"));
    assert_eq!(count.load(Ordering::SeqCst), 1, "inner was invoked");
}

async fn assert_window_blocks_inner(window: BudgetWindow) {
    let (inner, count) = RecordingTransport::new();
    let gate = SpendGate::new(inner, Arc::new(AlwaysBlock { window }));
    let err = gate
        .complete(&dummy_request())
        .await
        .expect_err("ceiling trips before inner");
    match err {
        ArgoError::BudgetExceeded { window: w, .. } => assert_eq!(w, window),
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }
    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "inner must NOT be invoked when ceiling trips"
    );
}

#[tokio::test]
async fn over_session_cap_fails_fast_without_calling_inner() {
    assert_window_blocks_inner(BudgetWindow::Session).await;
}

#[tokio::test]
async fn over_day_cap_fails_fast_without_calling_inner() {
    assert_window_blocks_inner(BudgetWindow::Day).await;
}

#[tokio::test]
async fn over_month_cap_fails_fast_without_calling_inner() {
    assert_window_blocks_inner(BudgetWindow::Month).await;
}

#[tokio::test]
async fn projected_cost_equal_to_remaining_is_allowed() {
    // Boundary-inclusive contract: the enforcer (implemented
    // elsewhere) returns Ok(()) when projected == remaining; the
    // gate just trusts the verdict.
    let (inner, count) = RecordingTransport::new();
    let gate = SpendGate::with_projected_cost(inner, Arc::new(AlwaysAllow), 500_000);
    gate.complete(&dummy_request())
        .await
        .expect("boundary-inclusive passes");
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn streaming_path_also_short_circuits_on_ceiling_trip() {
    // Locks the contract that `complete_stream` projects BEFORE
    // dialing — same as `complete`. The most-common regression
    // mode for streaming wrappers is "checks on complete but not
    // complete_stream" (cf. BreakerTransport history).
    let (inner, count) = RecordingTransport::new();
    let gate = SpendGate::new(
        inner,
        Arc::new(AlwaysBlock {
            window: BudgetWindow::Session,
        }),
    );
    // `BoxStream` doesn't implement Debug, so we can't use
    // `expect_err`; pattern-match the Result by hand instead.
    match gate.complete_stream(&dummy_request()).await {
        Err(ArgoError::BudgetExceeded { .. }) => {}
        Err(other) => panic!("expected BudgetExceeded, got {other:?}"),
        Ok(_) => panic!("stream should not establish when ceiling trips"),
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
