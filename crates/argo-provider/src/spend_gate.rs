//! Enforcement-time hard `[budget.ceiling]` wrapper —
//! `SpendGate<T: Transport>`. Sits between the agent loop and the
//! concrete provider transport; pre-projects the per-call cost
//! against the configured per-session / per-day / per-month
//! ceilings and **fails before invoking the inner transport** when
//! any window would be exceeded.
//!
//! # Shape
//!
//! Mirrors [`crate::transport::BreakerTransport`]: thin wrapper, no
//! request-shape mutation, full `Transport` re-impl. The read side
//! (where the ledger lives) is provided through the
//! [`BudgetEnforcer`] trait so this crate stays free of an
//! `argo-store` dependency. Production callers wire a
//! `SessionStore`-backed enforcer in via the follow-up routing brief
//! `mission-budget-gate-route-callers`.
//!
//! # Projected cost
//!
//! Today `ProviderRequest` does not carry a per-call cost estimate,
//! so the gate uses
//! [`argo_store::spend_ceiling::FALLBACK_PROJECTED_COST_MICROS`]
//! (4¢) — *conservative* on purpose: better to flunk one cheap
//! request than to allow a runaway charge past the cap.
//!
//! # Errors
//!
//! On overshoot, returns [`argo_types::ArgoError::BudgetExceeded`]
//! with the **first** tripped window (Session → Day → Month order)
//! WITHOUT invoking the inner transport. Pass-through when all
//! ceilings are uncapped or every window has headroom.
use crate::transport::{ProviderRequest, ProviderResponse, ProviderResponseDelta, Transport};
use argo_types::{
    AnomalyAlert, AnomalyDispatch, AnomalyKind, ArgoError, ArgoResult, CapabilitySet, Severity,
};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Conservative ceiling-side projected-cost fallback (USD micros) —
/// 4¢ per request. Re-exported alongside
/// [`argo_store::spend_ceiling::FALLBACK_PROJECTED_COST_MICROS`] so
/// callers that don't depend on `argo-store` (tests, gateway shims)
/// can name the same constant.
pub const FALLBACK_PROJECTED_COST_MICROS: u64 = 40_000;

/// Read side for [`SpendGate`] — invoked once per `complete` /
/// `complete_stream` to decide whether the projected cost would
/// trip any configured `[budget.ceiling]` cap.
///
/// Implementors capture the tenant + session + ceiling-config they
/// were constructed for; the trait method takes only the projected
/// cost so the gate has zero coupling to the underlying store.
///
/// The contract:
///
/// - Return `Ok(())` to **pass through** to the wrapped transport.
/// - Return `Err(ArgoError::BudgetExceeded { ... })` to **block**
///   the call without invoking the inner transport.
///
/// The production impl lives outside this crate (constructed at
/// session-creation time and bound to a `SessionStore` + a parsed
/// [`argo-config::budget_ceiling`] config); a stub for tests is
/// `static_enforcer` below.
///
/// ```
/// use argo_provider::spend_gate::BudgetEnforcer;
/// use argo_types::{ArgoError, ArgoResult, BudgetWindow};
/// use async_trait::async_trait;
///
/// struct AlwaysAllow;
/// #[async_trait]
/// impl BudgetEnforcer for AlwaysAllow {
///     async fn enforce(&self, _projected_cost_micros: u64) -> ArgoResult<()> {
///         Ok(())
///     }
/// }
/// let _: Box<dyn BudgetEnforcer> = Box::new(AlwaysAllow);
/// # let _ = BudgetWindow::Session;
/// # let _: ArgoError = ArgoError::Interrupted;
/// ```
#[async_trait]
pub trait BudgetEnforcer: Send + Sync {
    /// Project a charge of `projected_cost_micros` against the
    /// configured ceilings.
    ///
    /// # Errors
    /// Returns [`argo_types::ArgoError::BudgetExceeded`] with the
    /// first tripped window when any ceiling would be exceeded.
    async fn enforce(&self, projected_cost_micros: u64) -> ArgoResult<()>;
}

/// [`Transport`] wrapper that enforces hard `[budget.ceiling]` caps
/// at provider-call time. Construct one per `(tenant, session)`
/// pair — the [`BudgetEnforcer`] captures the scope it was bound to.
///
/// ```
/// # async fn example<T: argo_provider::Transport>(inner: T) {
/// use std::sync::Arc;
/// use argo_provider::spend_gate::{BudgetEnforcer, SpendGate};
/// use argo_types::ArgoResult;
/// use async_trait::async_trait;
///
/// struct Noop;
/// #[async_trait]
/// impl BudgetEnforcer for Noop {
///     async fn enforce(&self, _p: u64) -> ArgoResult<()> { Ok(()) }
/// }
/// let _gate = SpendGate::new(inner, Arc::new(Noop));
/// # }
/// ```
pub struct SpendGate<T: Transport> {
    inner: T,
    enforcer: Arc<dyn BudgetEnforcer>,
    projected_cost_micros: u64,
    anomaly_dispatch: Option<Arc<dyn AnomalyDispatch>>,
}

impl<T: Transport> SpendGate<T> {
    /// Wrap `inner` with `enforcer`. Each call gets projected against
    /// [`FALLBACK_PROJECTED_COST_MICROS`].
    pub fn new(inner: T, enforcer: Arc<dyn BudgetEnforcer>) -> Self {
        Self {
            inner,
            enforcer,
            projected_cost_micros: FALLBACK_PROJECTED_COST_MICROS,
            anomaly_dispatch: None,
        }
    }

    /// Variant that overrides the projected per-call cost (callers
    /// that have a tighter estimate than the conservative 4¢
    /// fallback can plug it in here — used by tests and by the
    /// follow-up routing brief once `ProviderRequest` grows an
    /// `estimated_cost_micros` hint).
    pub fn with_projected_cost(
        inner: T,
        enforcer: Arc<dyn BudgetEnforcer>,
        projected_cost_micros: u64,
    ) -> Self {
        Self {
            inner,
            enforcer,
            projected_cost_micros,
            anomaly_dispatch: None,
        }
    }

    /// Attach anomaly dispatch for hard budget-ceiling hits.
    #[must_use]
    pub fn with_anomaly_dispatch(mut self, dispatch: Arc<dyn AnomalyDispatch>) -> Self {
        self.anomaly_dispatch = Some(dispatch);
        self
    }

    /// Borrowed inner transport (used by the routing brief to
    /// compose `SpendGate` over an already-wrapped `BreakerTransport`).
    pub fn inner(&self) -> &T {
        &self.inner
    }

    async fn emit_budget_ceiling(&self, err: &ArgoError) {
        let Some(dispatch) = self.anomaly_dispatch.as_ref() else {
            return;
        };
        let ArgoError::BudgetExceeded {
            window,
            used_micros,
            ceiling_micros,
            projected_cost_micros,
        } = err
        else {
            return;
        };
        let mut context = BTreeMap::new();
        context.insert("provider_id".into(), self.inner.id().to_string());
        context.insert("window".into(), format!("{window:?}"));
        context.insert("used_micros".into(), used_micros.to_string());
        context.insert("ceiling_micros".into(), ceiling_micros.to_string());
        context.insert(
            "projected_cost_micros".into(),
            projected_cost_micros.to_string(),
        );
        let alert = AnomalyAlert {
            kind: AnomalyKind::BudgetCeiling,
            severity: Severity::Warn,
            message: format!(
                "budget ceiling blocked provider call to {}",
                self.inner.id()
            ),
            context,
        };
        if let Err(e) = dispatch.dispatch(alert).await {
            tracing::warn!(target: "provider::spend_gate", error = %e, "budget ceiling anomaly dispatch failed");
        }
    }
}

#[async_trait]
impl<T: Transport> Transport for SpendGate<T> {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn capabilities(&self) -> &CapabilitySet {
        self.inner.capabilities()
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        if let Err(e) = self.enforcer.enforce(self.projected_cost_micros).await {
            self.emit_budget_ceiling(&e).await;
            return Err(e);
        }
        self.inner.complete(req).await
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        // Same contract as `complete`: project the charge BEFORE
        // establishing the stream so we don't burn the open-connect
        // cost on a doomed-to-fail call.
        if let Err(e) = self.enforcer.enforce(self.projected_cost_micros).await {
            self.emit_budget_ceiling(&e).await;
            return Err(e);
        }
        self.inner.complete_stream(req).await
    }
}

#[cfg(test)]
#[path = "spend_gate_tests.rs"]
mod tests;
