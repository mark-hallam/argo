//! Async lookup hook for `TargetSelector::SkillMatch` and
//! `TargetSelector::RoleMatch` resolution (Stream
//! `mission-routing-skillmatch-rolematch`, Tier M4).
//!
//! The orchestrator stays free of `argo-cli` / `argo-skill` runtime
//! deps by declaring this trait here and letting `argo-cli` host the
//! production implementation. Tests inside this crate construct
//! lightweight stubs to exercise [`RoutingEngine::route_async`]
//! without spinning up a real `IdentityStore`.
//!
//! ## Ranking contract
//!
//! Both impls must rank candidates by `trust_grade DESC,
//! last_used_at DESC, worker_id ASC`. [`rank_workers`] is the shared
//! helper that codifies this ordering so resolver impls don't drift.

use argo_identity::WorkerId;
use argo_types::{ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;

/// Hook the routing engine calls when a rule's
/// [`TargetSelector`](crate::routing::TargetSelector) is
/// `SkillMatch(String)` or `RoleMatch(String)`. The thin production
/// implementation lives in `argo-cli`; the orchestrator never depends
/// on `argo-skill` directly.
///
/// Returning `Ok(None)` is normal — the routing engine falls through
/// to the explicit escalation chain target. Errors are swallowed by
/// the engine and treated as `None` so a resolver fault never crashes
/// the walker tick (the underlying `ArgoError` surfaces via the
/// resolver's own `tracing` if it instruments).
#[async_trait::async_trait]
pub trait TargetResolver: Send + Sync {
    /// Resolve a `SkillMatch(skill_name)` selector to a concrete
    /// worker. Implementations look up the worker↔skill table for
    /// `tenant`, rank candidates per the contract documented at the
    /// trait level, and return the top match.
    async fn resolve_skill_match(
        &self,
        tenant: TenantId,
        skill_name: &str,
    ) -> ArgoResult<Option<WorkerId>>;

    /// Resolve a `RoleMatch(role_name)` selector to a concrete
    /// worker. Implementations look up the operator-stamped
    /// `worker.role_name` column for `tenant`, rank candidates per
    /// the contract documented at the trait level, and return the
    /// top match.
    async fn resolve_role_match(
        &self,
        tenant: TenantId,
        role_name: &str,
    ) -> ArgoResult<Option<WorkerId>>;
}

/// Rank `(worker_id, trust_grade, last_used_at)` candidates by
/// `trust_grade DESC, last_used_at DESC, worker_id ASC`, returning
/// the top match. Empty input → `None`.
///
/// Determinism: two workers tied on grade + recency tie-break on
/// `worker_id ASC` (lower UUID wins). UUIDv7's monotone timestamp
/// prefix means the first-created worker wins under a recency tie,
/// which matches the brief's "deterministic-by-id" semantics.
///
/// ```
/// use argo_orchestrator::rank_workers;
/// use argo_identity::WorkerId;
/// let a = WorkerId::nil();
/// let b = WorkerId::new();
/// // Higher trust_grade wins outright.
/// assert_eq!(rank_workers(&[(a, 1, None), (b, 5, None)]), Some(b));
/// // Empty input → None.
/// assert_eq!(rank_workers(&[]), None);
/// ```
#[must_use]
pub fn rank_workers(
    candidates: &[(WorkerId, u8, Option<DateTime<Utc>>)],
) -> Option<WorkerId> {
    candidates
        .iter()
        .max_by(|a, b| compare_candidates(a, b))
        .map(|(id, _, _)| *id)
}

fn compare_candidates(
    a: &(WorkerId, u8, Option<DateTime<Utc>>),
    b: &(WorkerId, u8, Option<DateTime<Utc>>),
) -> Ordering {
    a.1.cmp(&b.1)
        .then_with(|| a.2.cmp(&b.2))
        // worker_id ASC: lower id should rank higher in max_by, so
        // reverse the natural Uuid ordering here.
        .then_with(|| b.0 .0.cmp(&a.0 .0))
}
