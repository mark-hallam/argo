//! Quiet-hours window check extracted from
//! [`super::worker_actions_auto_approval_gates::apply_effective_gates`].
//!
//! Returns `Ok(None)` when no `quiet_hours` window is configured or the
//! current instant is outside the window — the caller keeps its prior
//! decision. Returns `Ok(Some(Ask{..}))` for
//! `QuietHoursBehavior::QueueUntilMorning` and `Err(..)` for
//! `QuietHoursBehavior::Block`, so the gate downgrade / fail-closed
//! semantics live in one place.
//!
//! Timezone source: no tenant timezone surface exists yet, so quiet-
//! hours windows compare against UTC. Operators setting `quiet_hours`
//! should write windows in UTC until tenant-tz wiring lands (tracked
//! in the F-06 follow-up).

use argo_identity::auto_approval::AutoApprovalDecision;
use argo_identity::policy::{EffectivePolicy, QuietHoursBehavior};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};

use super::worker_actions_auto_approval::block_to_error;

pub(super) fn quiet_hours_check(
    action_id: &str,
    effective: &EffectivePolicy,
    now: DateTime<Utc>,
) -> ArgoResult<Option<AutoApprovalDecision>> {
    let Some(qh) = effective.quiet_hours.as_ref() else {
        return Ok(None);
    };
    let active = qh
        .is_active_at(now.time())
        .map_err(|e| ArgoError::Provider(format!("quiet_hours time parse failed: {e}")))?;
    if !active {
        return Ok(None);
    }
    match qh.behavior {
        QuietHoursBehavior::Block => Err(block_to_error(action_id, "quiet_hours active")),
        QuietHoursBehavior::QueueUntilMorning => Ok(Some(AutoApprovalDecision::Ask {
            channel: None,
            reason: "queued: quiet_hours active".into(),
        })),
    }
}
