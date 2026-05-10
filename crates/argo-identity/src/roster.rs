//! V1.3 — Worker rosters: group-level policy + channels + budget.
//!
//! A [`Roster`] is a named group of workers that share an
//! [`AutoApprovalPolicy`], a list of [`NotifyChannelRef`]s, and an
//! optional spend cap. Per-worker policy resolution
//! ([`crate::Worker::resolved_policy_with_rosters`]) consults the
//! rosters a worker belongs to whenever the worker's own
//! `auto_approval` is disabled — workspace defaults are the final
//! fallback below the roster layer.
//!
//! Multi-tenant: every primitive carries `tenant_id`. Two tenants
//! can have completely separate rosters in the same DB without
//! leakage.
//!
//! Conflict resolution (when a worker belongs to multiple rosters)
//! is "most restrictive wins" — see
//! [`AutoApprovalPolicy::most_restrictive`].
//!
//! Persistence (`RosterStore`) lives in `roster_store.rs` and is
//! re-exported here so the public path
//! `argo_identity::roster::RosterStore` is unchanged.

use crate::types::{AutoApprovalPolicy, NotifyChannelRef, WorkerId};
use argo_types::TenantId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::roster_store::RosterStore;

/// Stable roster id (UUIDv7).
///
/// ```
/// use argo_identity::roster::RosterId;
/// let id = RosterId::new();
/// assert_eq!(id.0.get_version_num(), 7);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RosterId(pub Uuid);

impl RosterId {
    /// Generate a fresh id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for RosterId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RosterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::str::FromStr for RosterId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// Group of workers that share auto-approval policy + notification
/// channels + an optional spend cap.
///
/// ```
/// use argo_identity::roster::Roster;
/// use argo_identity::AutoApprovalPolicy;
/// use argo_types::TenantId;
///
/// let roster = Roster::new(
///     TenantId::new(),
///     "sales-ops",
///     AutoApprovalPolicy::default(),
///     Vec::new(),
///     Some(50.0),
/// );
/// assert_eq!(roster.name, "sales-ops");
/// assert_eq!(roster.budget_cap_usd, Some(50.0));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roster {
    /// Stable roster id.
    pub id: RosterId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Operator-friendly group name (unique per tenant).
    pub name: String,
    /// Workers that belong to this roster.
    pub member_worker_ids: Vec<WorkerId>,
    /// Shared auto-approval policy applied when a member's own
    /// `auto_approval` is disabled.
    pub policy: AutoApprovalPolicy,
    /// Notification channel preferences for the group. Member
    /// workers without explicit channels fall back to these.
    pub channels: Vec<NotifyChannelRef>,
    /// Optional group-level spend ceiling, expressed in whole USD
    /// (operator-facing units). Independent of the per-action /
    /// per-day micros caps inside `policy`.
    pub budget_cap_usd: Option<f64>,
}

impl Roster {
    /// Build a fresh roster with no members. Members are added via
    /// [`RosterStore::add_member`] after persistence.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        name: impl Into<String>,
        policy: AutoApprovalPolicy,
        channels: Vec<NotifyChannelRef>,
        budget_cap_usd: Option<f64>,
    ) -> Self {
        Self {
            id: RosterId::new(),
            tenant_id,
            name: name.into(),
            member_worker_ids: Vec::new(),
            policy,
            channels,
            budget_cap_usd,
        }
    }
}

impl AutoApprovalPolicy {
    /// Merge a slice of policies into the most-restrictive
    /// resolved policy.
    ///
    /// "Most restrictive" semantics:
    /// * `enabled` — the merged policy is enabled iff at least one
    ///   input policy is enabled. Callers normally pre-filter to
    ///   enabled inputs (the resolver does); the safe fallback
    ///   when the slice is empty is the default disabled policy.
    /// * `max_usd_micros_per_action` — minimum of all set values
    ///   (any unset = no constraint, so it's skipped).
    /// * `daily_budget_usd_micros` — minimum of all set values.
    /// * `time_windows` — keep the shortest non-empty list (fewest
    ///   allowances = strictest). If every input has an empty list
    ///   the merged list is empty (no time-based restriction).
    /// * `wake_action_types` — intersection (a type is wake-allowed
    ///   only if every input policy lists it).
    ///
    /// ```
    /// use argo_identity::AutoApprovalPolicy;
    /// let a = AutoApprovalPolicy {
    ///     enabled: true,
    ///     daily_budget_usd_micros: Some(10_000),
    ///     ..Default::default()
    /// };
    /// let b = AutoApprovalPolicy {
    ///     enabled: true,
    ///     daily_budget_usd_micros: Some(5_000),
    ///     ..Default::default()
    /// };
    /// let merged = AutoApprovalPolicy::most_restrictive(&[&a, &b]);
    /// assert_eq!(merged.daily_budget_usd_micros, Some(5_000));
    /// ```
    #[must_use]
    pub fn most_restrictive(policies: &[&AutoApprovalPolicy]) -> AutoApprovalPolicy {
        if policies.is_empty() {
            return AutoApprovalPolicy::default();
        }
        let enabled = policies.iter().any(|p| p.enabled);
        let max_per_action = policies
            .iter()
            .filter_map(|p| p.max_usd_micros_per_action)
            .min();
        let daily_budget = policies
            .iter()
            .filter_map(|p| p.daily_budget_usd_micros)
            .min();
        let time_windows = policies
            .iter()
            .map(|p| p.time_windows.clone())
            .filter(|w| !w.is_empty())
            .min_by_key(std::vec::Vec::len)
            .unwrap_or_default();
        let wake_action_types = intersection_of_string_lists(
            &policies
                .iter()
                .map(|p| p.wake_action_types.as_slice())
                .collect::<Vec<&[String]>>(),
        );
        AutoApprovalPolicy {
            enabled,
            max_usd_micros_per_action: max_per_action,
            daily_budget_usd_micros: daily_budget,
            time_windows,
            wake_action_types,
        }
    }
}

fn intersection_of_string_lists(lists: &[&[String]]) -> Vec<String> {
    let Some((head, tail)) = lists.split_first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in *head {
        if tail.iter().all(|other| other.contains(item)) {
            out.push(item.clone());
        }
    }
    out
}

#[cfg(test)]
#[path = "roster_tests.rs"]
mod tests;
