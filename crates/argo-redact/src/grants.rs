//! Elevated-view grant lookup.
//!
//! A grant is a per-principal, per-category, time-bounded "show me
//! the raw value" override. The DB row is owned by J16
//! (`gateway-elevated-view-request-endpoint`), which has not landed
//! yet — v1 mocks the lookup as always-empty so the fence
//! short-circuits to `viewer.permissions` alone. When J16 lands it
//! swaps in the real `SELECT category, expires_at FROM
//! elevated_view_grant WHERE principal_id = $1 AND expires_at >
//! NOW()` reader behind the same function signature.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::patterns::Category;

/// One row from the `elevated_view_grant` table. `id` is the surrogate
/// key persisted alongside (so the audit payload can reference the
/// specific grant the viewer used).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// Surrogate key. Emitted into `audit.grant_id` when this grant
    /// is responsible for showing the raw value.
    pub id: String,
    /// Which category this grant covers.
    pub category: Category,
    /// Wall-clock expiry. Grants whose `expires_at` is in the past
    /// are filtered out by [`GrantSet::active`].
    pub expires_at: DateTime<Utc>,
}

/// A bag of grants for a single principal. The constructor filters
/// expired rows, so callers can treat every grant inside as live.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrantSet {
    grants: Vec<Grant>,
}

impl GrantSet {
    /// Empty grant set — the v1 mock and the test "no grants" path.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build a set from the raw rows pulled from the DB. `now`
    /// drives expiry filtering: any grant whose `expires_at <= now`
    /// is dropped.
    #[must_use]
    pub fn active(rows: impl IntoIterator<Item = Grant>, now: DateTime<Utc>) -> Self {
        Self {
            grants: rows.into_iter().filter(|g| g.expires_at > now).collect(),
        }
    }

    /// True when at least one grant in the set covers `category`.
    #[must_use]
    pub fn covers(&self, category: Category) -> bool {
        self.grants.iter().any(|g| g.category == category)
    }

    /// Return the surrogate id of the first grant covering
    /// `category`, for the audit payload's `grant_id` field. `None`
    /// if no grant covers it.
    #[must_use]
    pub fn id_for(&self, category: Category) -> Option<&str> {
        self.grants
            .iter()
            .find(|g| g.category == category)
            .map(|g| g.id.as_str())
    }

    /// Return the id of the first grant in the set, irrespective of
    /// category. Used by the audit emitter when the viewer was
    /// shown one-or-more categories via the same grant bundle.
    #[must_use]
    pub fn first_id(&self) -> Option<&str> {
        self.grants.first().map(|g| g.id.as_str())
    }

    /// True when the set has no live grants.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }
}

/// Mock grant lookup for v1. Always returns an empty set; the real
/// DB-backed reader lands with J16. The signature is stable across
/// the v1 → v1.5 swap so callers don't change.
#[must_use]
pub fn active_grants_for_principal(_principal_id: &str) -> GrantSet {
    GrantSet::empty()
}
