//! Duty fire-decision enum + approval-gate trait.
//!
//! Extracted from `duty_scheduler.rs` (mvp-split-duty-scheduler);
//! re-exported under that module for backward compatibility.
use crate::duty_scheduler::DutyRecord;
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Decision rendered by the AutoApprovalPolicy gate when a duty
/// fire is evaluated. Mirrors the `decision` column in
/// `duty_fired_session`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireDecision {
    /// Gate allowed the fire to spawn a session.
    AutoApprove,
    /// Gate denied (worker policy, budget, time-window, etc.).
    DeniedGate,
    /// Duty was paused at fire time.
    DeniedPaused,
    /// Duty was disabled at fire time.
    DeniedDisabled,
    /// Manual `argo duty fire-now` invocation (bypasses cron).
    ManualFire,
}

impl FireDecision {
    /// Stable label written to the link table.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::AutoApprove => "auto_approved",
            Self::DeniedGate => "denied_gate",
            Self::DeniedPaused => "denied_paused",
            Self::DeniedDisabled => "denied_disabled",
            Self::ManualFire => "manual_fire",
        }
    }
}

/// Approval gate consulted before a duty fire becomes a Session.
/// Production callers wire this to the H-1 evaluator over the
/// worker's resolved `AutoApprovalPolicy`; tests use a stub.
#[async_trait]
pub trait DutyGate: Send + Sync {
    /// Decide whether the duty's pending fire should proceed.
    async fn evaluate(&self, duty: &DutyRecord) -> ArgoResult<FireDecision>;
}

/// `DutyGate` that auto-approves every fire — gateway-default
/// when no per-worker policy is configured.
pub struct OpenGate;

#[async_trait]
impl DutyGate for OpenGate {
    async fn evaluate(&self, _duty: &DutyRecord) -> ArgoResult<FireDecision> {
        Ok(FireDecision::AutoApprove)
    }
}

/// `DutyGate` that denies every fire — useful in tests for proving
/// the gate-honored acceptance criterion.
pub struct ClosedGate;

#[async_trait]
impl DutyGate for ClosedGate {
    async fn evaluate(&self, _duty: &DutyRecord) -> ArgoResult<FireDecision> {
        Ok(FireDecision::DeniedGate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::WorkerId;
    use argo_types::TenantId;

    fn record(cron: &str) -> DutyRecord {
        DutyRecord::new(TenantId::new(), WorkerId::new(), cron, "ping", "test duty")
    }

    #[tokio::test]
    async fn open_gate_auto_approves() {
        let g = OpenGate;
        assert_eq!(
            g.evaluate(&record("0 * * * * *")).await.unwrap(),
            FireDecision::AutoApprove
        );
    }

    #[tokio::test]
    async fn closed_gate_denies() {
        let g = ClosedGate;
        assert_eq!(
            g.evaluate(&record("0 * * * * *")).await.unwrap(),
            FireDecision::DeniedGate
        );
    }

    #[tokio::test]
    async fn fire_decision_labels_are_stable() {
        assert_eq!(FireDecision::AutoApprove.label(), "auto_approved");
        assert_eq!(FireDecision::DeniedGate.label(), "denied_gate");
        assert_eq!(FireDecision::DeniedPaused.label(), "denied_paused");
        assert_eq!(FireDecision::DeniedDisabled.label(), "denied_disabled");
        assert_eq!(FireDecision::ManualFire.label(), "manual_fire");
    }
}
