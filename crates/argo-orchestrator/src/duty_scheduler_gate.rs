//! Duty fire-decision enum + approval-gate trait.
//!
//! Extracted from `duty_scheduler.rs` (mvp-split-duty-scheduler);
//! re-exported under that module for backward compatibility.
use crate::duty_scheduler::DutyRecord;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use chrono::{Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

/// Per-duty gate selection persisted alongside the duty config.
///
/// `Open` is the default and delegates to the [`DutyScheduler`]'s
/// scheduler-level gate (typically [`OpenGate`] in production), which
/// preserves today's behavior for legacy rows that don't carry the
/// field. The other variants override the scheduler-level gate with
/// a per-duty rule.
///
/// `Schedule` and `QuietHours` overlap by design: `Schedule` says
/// "fire IFF current hour ∈ allow", `QuietHours` says "fire UNLESS
/// current hour ∈ deny". Operators pick whichever matches their
/// mental model. Hour granularity is intentional.
///
/// ```
/// use argo_orchestrator::duty_scheduler::GateConfig;
/// let cfg = GateConfig::QuietHours {
///     deny_hours: vec![22, 23, 0, 1, 2, 3, 4, 5, 6, 7],
///     tz: "Australia/Melbourne".into(),
/// };
/// assert!(matches!(cfg, GateConfig::QuietHours { .. }));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum GateConfig {
    /// Delegate to the scheduler-level gate (today's behavior).
    #[default]
    Open,
    /// Always deny — duty is configured-off.
    Closed,
    /// Fire only when the current UTC hour is in `allow_hours`.
    Schedule {
        /// Hours (0..=23) during which fires are allowed.
        allow_hours: Vec<u8>,
    },
    /// Fire unless the current local hour (in `tz`) is in `deny_hours`.
    QuietHours {
        /// Hours (0..=23) during which fires are denied.
        deny_hours: Vec<u8>,
        /// IANA timezone (e.g. "Australia/Melbourne") parsed via `chrono-tz`.
        tz: String,
    },
}

/// `DutyGate` that fires only when the current UTC hour is in
/// `allow_hours`. Built by [`gate_from_config`] from a
/// `GateConfig::Schedule` variant.
pub struct ScheduleGate {
    allow_hours: Vec<u8>,
}

impl ScheduleGate {
    /// Build a gate from an explicit allow-list of hours (0..=23).
    #[must_use]
    pub fn new(allow_hours: Vec<u8>) -> Self {
        Self { allow_hours }
    }

    fn allows(&self, hour: u8) -> bool {
        self.allow_hours.contains(&hour)
    }
}

#[async_trait]
impl DutyGate for ScheduleGate {
    async fn evaluate(&self, _duty: &DutyRecord) -> ArgoResult<FireDecision> {
        let hour = u8::try_from(Utc::now().hour()).unwrap_or(0);
        if self.allows(hour) {
            Ok(FireDecision::AutoApprove)
        } else {
            Ok(FireDecision::DeniedGate)
        }
    }
}

/// `DutyGate` that denies fires when the current local hour (in
/// `tz`) is in `deny_hours`. Built by [`gate_from_config`] from a
/// `GateConfig::QuietHours` variant.
pub struct QuietHoursGate {
    deny_hours: Vec<u8>,
    tz: chrono_tz::Tz,
}

impl QuietHoursGate {
    /// Build a gate from a deny-list of hours (0..=23) and a parsed
    /// IANA timezone.
    #[must_use]
    pub fn new(deny_hours: Vec<u8>, tz: chrono_tz::Tz) -> Self {
        Self { deny_hours, tz }
    }

    fn denies(&self, hour: u8) -> bool {
        self.deny_hours.contains(&hour)
    }
}

#[async_trait]
impl DutyGate for QuietHoursGate {
    async fn evaluate(&self, _duty: &DutyRecord) -> ArgoResult<FireDecision> {
        let local = Utc::now().with_timezone(&self.tz);
        let hour = u8::try_from(local.hour()).unwrap_or(0);
        if self.denies(hour) {
            Ok(FireDecision::DeniedGate)
        } else {
            Ok(FireDecision::AutoApprove)
        }
    }
}

/// Build a boxed [`DutyGate`] from a [`GateConfig`].
///
/// Returns an error for `QuietHours` with an unparseable IANA
/// timezone — surfaced at config-load time rather than at gate-tick.
/// `Open` returns [`OpenGate`] for callers that want the literal
/// default; the scheduler's per-duty resolution layers a fallback to
/// the scheduler-level gate on top of this.
pub fn gate_from_config(cfg: &GateConfig) -> ArgoResult<Box<dyn DutyGate>> {
    match cfg {
        GateConfig::Open => Ok(Box::new(OpenGate)),
        GateConfig::Closed => Ok(Box::new(ClosedGate)),
        GateConfig::Schedule { allow_hours } => Ok(Box::new(ScheduleGate::new(allow_hours.clone()))),
        GateConfig::QuietHours { deny_hours, tz } => {
            let parsed = chrono_tz::Tz::from_str(tz)
                .map_err(|e| ArgoError::Schema(format!("gate.quiet_hours.tz '{tz}': {e}")))?;
            Ok(Box::new(QuietHoursGate::new(deny_hours.clone(), parsed)))
        }
    }
}

#[cfg(test)]
#[path = "duty_scheduler_gate_tests.rs"]
mod gate_tests;

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
