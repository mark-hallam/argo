//! Stream H-1 — Auto-approval policy evaluator.
//!
//! Pure decision engine: takes an action + worker policy +
//! recent spend → [`Decision`]. The downstream gate (Stream H-2,
//! `worker_actions_dispatch::execute`) consults the evaluator
//! before notifying any channel. When the verdict is
//! `AutoApprove`, the action is approved with `approved_by =
//! "auto:<policy>"`; when it's `Ask(channel)`, the existing
//! channel notify path fires; when it's `Block`, the action
//! lands `blocked` with the reason.
//!
//! The evaluator is offline + deterministic — no network, no
//! Claude calls. All policy decisions trace back to the
//! [`AutoApprovalPolicy`] declaration on the [`crate::Worker`]
//! plus the [`SpendSnapshot`] the budget ledger ships.

use crate::types::AutoApprovalPolicy;
use chrono::{DateTime, Datelike, NaiveTime, Utc, Weekday};
use serde::{Deserialize, Serialize};

/// Action context the evaluator inspects. Caller (the dispatch
/// shim) builds this from the `ActionRecord` it's about to
/// execute. Fields are denormalised so the evaluator stays a
/// pure function of its inputs.
#[derive(Debug, Clone)]
pub struct PendingAction<'a> {
    /// The action's `risk_level` field — `low | medium | high`.
    pub risk_level: &'a str,
    /// The action's `requires_approval` declaration. When false
    /// the policy still runs (cheap), but the verdict is always
    /// `AutoApprove` for the autonomous-safe path.
    pub requires_approval: bool,
    /// Optional per-action USD-micros estimate. The dispatch
    /// shim sets this from the executor declaration's
    /// `budget_usd` field; `None` means the action is free.
    pub estimated_usd_micros: Option<u64>,
    /// Action type as declared in the manifest. Used for
    /// per-kind allowlists in future iterations of the policy.
    pub action_type: &'a str,
}

/// Snapshot of the worker's spend ledger AT EVALUATION TIME.
/// Built by [`crate::worker_budget::WorkerBudget::snapshot`];
/// passed by value because the evaluator must not mutate it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendSnapshot {
    /// USD micros spent in the current calendar day (UTC).
    pub today_usd_micros: u64,
    /// USD micros spent in the current calendar month (UTC).
    pub this_month_usd_micros: u64,
}

impl SpendSnapshot {
    /// Empty ledger (used in tests + when a brand-new worker
    /// hasn't spent anything yet).
    #[must_use]
    pub fn zero() -> Self {
        Self {
            today_usd_micros: 0,
            this_month_usd_micros: 0,
        }
    }
}

/// Verdict the gate honours. `Ask` carries the channel selector
/// the dispatch shim should use — `None` means "default channel
/// list from the worker's `channels` field".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// Action clears auto-approval. Dispatch writes
    /// `approved_by = "auto:<reason>"` and proceeds.
    AutoApprove {
        /// Stable reason string for audit logs.
        reason: String,
    },
    /// Action requires human approval. The optional channel id
    /// pins the notify path; `None` falls back to the worker's
    /// default channels.
    Ask {
        /// Channel id from the worker's `channels` list, or
        /// `None` to fan out across all defaults.
        channel: Option<String>,
        /// Stable reason string for audit logs.
        reason: String,
    },
    /// Action exceeds policy ceilings. Dispatch lands `blocked`
    /// with `reason` in the audit trail.
    Block {
        /// Stable reason string for audit logs.
        reason: String,
    },
}

impl Decision {
    /// Reason text — handy for tests and audit-log rendering.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::AutoApprove { reason } | Self::Ask { reason, .. } | Self::Block { reason } => {
                reason.as_str()
            }
        }
    }

    /// True when the verdict approves the action without human
    /// review. Used by the dispatch shim to decide whether to
    /// fire the channel-notify path.
    #[must_use]
    pub fn is_auto_approved(&self) -> bool {
        matches!(self, Self::AutoApprove { .. })
    }
}

/// Run the policy. Pure function — same inputs always produce
/// the same output. Caller is responsible for observing time
/// (passed via `now`) so tests pin it deterministically.
///
/// # Example
/// ```
/// use argo_identity::auto_approval::{
///     evaluate, Decision, PendingAction, SpendSnapshot,
/// };
/// use argo_identity::AutoApprovalPolicy;
/// use chrono::Utc;
/// let pending = PendingAction {
///     risk_level: "low",
///     requires_approval: false,
///     estimated_usd_micros: None,
///     action_type: "verify_price",
/// };
/// let decision = evaluate(
///     &AutoApprovalPolicy::default(),
///     &pending,
///     SpendSnapshot::zero(),
///     Utc::now(),
/// );
/// assert!(matches!(decision, Decision::AutoApprove { .. }));
/// ```
#[must_use]
pub fn evaluate(
    policy: &AutoApprovalPolicy,
    action: &PendingAction<'_>,
    spend: SpendSnapshot,
    now: DateTime<Utc>,
) -> Decision {
    // Autonomous-safe actions never need approval — the manifest
    // already says so. Skip the policy entirely.
    if !action.requires_approval {
        return Decision::AutoApprove {
            reason: "manifest:requires_approval=false".into(),
        };
    }

    // Policy disabled: every approval-gated action goes to a
    // human. Risk-class routing chooses the channel.
    if !policy.enabled {
        return Decision::Ask {
            channel: None,
            reason: "policy:disabled".into(),
        };
    }

    // High-risk actions never auto-approve. The brief's
    // safety hard rule: human-in-the-loop for every irreversible
    // / external-touching action.
    if action.risk_level == "high" {
        return Decision::Ask {
            channel: None,
            reason: "policy:high_risk_always_asks".into(),
        };
    }

    // Per-action ceiling: if the estimated spend exceeds the
    // cap, force an ask. None on the cap = no ceiling.
    if let (Some(estimate), Some(cap)) = (
        action.estimated_usd_micros,
        policy.max_usd_micros_per_action,
    ) {
        if estimate > cap {
            return Decision::Ask {
                channel: None,
                reason: format!(
                    "policy:per_action_cap=${:.2} exceeded by ${:.2}",
                    micros_to_dollars(cap),
                    micros_to_dollars(estimate),
                ),
            };
        }
    }

    // Daily ceiling: if today's spend + this estimate would
    // exceed the daily cap, BLOCK (don't ask — the operator
    // didn't agree to spend more today). None = unlimited.
    if let Some(daily_cap) = policy.daily_budget_usd_micros {
        let projected = spend
            .today_usd_micros
            .saturating_add(action.estimated_usd_micros.unwrap_or(0));
        if projected > daily_cap {
            return Decision::Block {
                reason: format!(
                    "policy:daily_cap=${:.2} would be exceeded \
                     (today=${:.2} + this=${:.2})",
                    micros_to_dollars(daily_cap),
                    micros_to_dollars(spend.today_usd_micros),
                    micros_to_dollars(action.estimated_usd_micros.unwrap_or(0)),
                ),
            };
        }
    }

    // Time-window check: if the policy declares allowed windows
    // and `now` falls outside all of them, force an ask. Empty
    // windows = no restriction. Wake-rules bypass the window
    // check so urgent action_types can clear quiet hours.
    let bypass_window = policy
        .wake_action_types
        .iter()
        .any(|w| w == action.action_type);
    if !bypass_window
        && !policy.time_windows.is_empty()
        && !window_matches(&policy.time_windows, now)
    {
        return Decision::Ask {
            channel: None,
            reason: "policy:outside_time_window".into(),
        };
    }

    // All gates passed. Auto-approve.
    Decision::AutoApprove {
        reason: format!("policy:risk={}_within_caps", action.risk_level),
    }
}

/// Convert USD micros to dollars for human-readable error
/// messages. Public so the dispatch shim can format
/// consistently.
#[must_use]
pub fn micros_to_dollars(micros: u64) -> f64 {
    (micros as f64) / 1_000_000.0
}

/// Convert dollars to USD micros (rounding to nearest micro).
/// Used by callers building policy values from operator-friendly
/// dollar inputs.
#[must_use]
pub fn dollars_to_micros(dollars: f64) -> u64 {
    (dollars * 1_000_000.0).max(0.0).round() as u64
}

/// One allowed time window. Days are weekdays the window covers;
/// `start` / `end` are local-clock UTC times. Cross-midnight
/// windows (start > end) wrap into the following day.
///
/// ```
/// use argo_identity::auto_approval::TimeWindow;
/// use chrono::{NaiveTime, Weekday};
/// let w = TimeWindow {
///     days: vec![Weekday::Mon, Weekday::Tue],
///     start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
///     end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
/// };
/// assert_eq!(w.days.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWindow {
    /// Days of the week the window applies to (UTC).
    pub days: Vec<Weekday>,
    /// Window start (UTC clock time).
    pub start: NaiveTime,
    /// Window end (UTC clock time). `end < start` means the
    /// window wraps past midnight into the following day.
    pub end: NaiveTime,
}

fn window_matches(windows: &[TimeWindow], now: DateTime<Utc>) -> bool {
    let weekday = now.weekday();
    let clock = now.time();
    windows.iter().any(|w| {
        if !w.days.contains(&weekday) {
            return false;
        }
        if w.start <= w.end {
            clock >= w.start && clock < w.end
        } else {
            // Wrap-past-midnight: matches if either before end
            // or after start.
            clock >= w.start || clock < w.end
        }
    })
}

#[cfg(test)]
#[path = "auto_approval_tests.rs"]
mod tests;
