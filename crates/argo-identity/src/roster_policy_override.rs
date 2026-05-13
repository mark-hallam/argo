//! Roster-scoped policy override — precursor for
//! `mission-roster-resolve-policy` v2.
//!
//! A roster carries its own [`crate::AutoApprovalPolicy`] (the
//! group baseline). A `RosterPolicyOverride` is a sparse,
//! all-`Option` shim layered ON TOP of that baseline so an
//! operator can raise/lower thresholds for a specific roster
//! without rebuilding the whole policy.
//!
//! This module ships ONLY the data type + a no-op `Default`. The
//! actual merge into [`crate::policy::EffectivePolicy`] is the
//! downstream `mission-roster-resolve-policy-v2` brief — that
//! brief enforces the manifest-as-floor invariant (override may
//! tighten but cannot widen what the workspace manifest allows).
//!
//! Persistence is a nullable `policy_override_json` column on the
//! `roster` table (migration v63). `None` means "no override"
//! (today's behaviour); `Some(RosterPolicyOverride)` means the
//! roster carries an override that the resolver should layer on
//! when computing the effective policy for one of its members.

use crate::policy::QuietHours;
use serde::{Deserialize, Serialize};

/// Sparse override for the per-roster policy. Every field is
/// `Option` so the operator can supply only the dimensions they
/// want to override; unset fields fall through to the roster's
/// own [`crate::AutoApprovalPolicy`] (and below that, the
/// workspace manifest).
///
/// ```
/// use argo_identity::RosterPolicyOverride;
/// let ov = RosterPolicyOverride::default();
/// assert!(ov.auto_approve_max_amount.is_none());
/// assert!(ov.additional_quiet_hours.is_none());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RosterPolicyOverride {
    /// Cap (in whole USD) on the auto-approval per-action limit.
    /// When set AND lower than the baseline, the resolver tightens
    /// the policy to this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_approve_max_amount: Option<u64>,
    /// Additional quiet-hours window layered on top of any
    /// existing workspace-level quiet-hours. Today this is an
    /// extra block; the v2 brief decides the union semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_quiet_hours: Option<QuietHours>,
}
