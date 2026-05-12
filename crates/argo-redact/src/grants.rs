//! Elevated-view grant lookup.
//!
//! A grant is a per-principal, per-category, time-bounded "show me
//! the raw value" override. Rows live in the workspace
//! `elevated_view_grant` table (owned by J16's migration v59); the
//! redact-policy resolver unions a viewer's grants with their RBAC
//! `unredacted_*` permissions before deciding what to mask.
//!
//! Boundary contract: argo-redact stays a leaf utility crate with
//! no SQL dependency. Callers load rows from `argo_store` and pass
//! them through [`GrantSet::from_store_rows`], which decodes the
//! wire-label category strings and filters out expired grants.
//! [`active_grants_for_principal`] is the v1 mock for callers that
//! have not yet been upgraded to the DB-backed path (it returns
//! empty so the redact fence falls back to `viewer.permissions`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::patterns::Category;

/// One per-category grant. The store row's `categories: Vec<String>`
/// fans out into N of these — the resolver works one category at a
/// time, so the explosion keeps the lookup branchless.
///
/// `id` is the surrogate key persisted in the store; it is emitted
/// into `audit.grant_id` when this grant is responsible for showing
/// the raw value.
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

/// A bag of grants for a single principal. The constructors filter
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

    /// Build a set from already-fanned-out per-category grants.
    /// `now` drives expiry filtering: any grant whose
    /// `expires_at <= now` is dropped.
    #[must_use]
    pub fn active(rows: impl IntoIterator<Item = Grant>, now: DateTime<Utc>) -> Self {
        Self {
            grants: rows.into_iter().filter(|g| g.expires_at > now).collect(),
        }
    }

    /// Decode store-shape rows (one row per `elevated_view_grant`
    /// table row, carrying a vector of wire-label category strings)
    /// into a [`GrantSet`]. Categories that don't map to a known
    /// [`Category`] variant are silently skipped — the J2 fence
    /// only enforces the v1 5-category set, while the handler may
    /// accept extra labels (e.g. `classification`, `approver`,
    /// `counterparty`) for forward compatibility with future
    /// categories.
    ///
    /// `now` drives expiry filtering: rows whose `expires_at <= now`
    /// are dropped before the category fan-out.
    pub fn from_store_rows<I, S>(rows: I, now: DateTime<Utc>) -> Self
    where
        I: IntoIterator<Item = (String, Vec<S>, DateTime<Utc>)>,
        S: AsRef<str>,
    {
        let mut grants = Vec::new();
        for (id, category_labels, expires_at) in rows {
            if expires_at <= now {
                continue;
            }
            for label in category_labels {
                if let Some(cat) = Category::from_label(label.as_ref()) {
                    grants.push(Grant {
                        id: id.clone(),
                        category: cat,
                        expires_at,
                    });
                }
            }
        }
        Self { grants }
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

/// v1 mock grant lookup. Returns an empty set so callers that have
/// not yet been wired to the DB-backed path fall through to the
/// `viewer.permissions` branch of the redact fence. New code should
/// load grants from `argo_store::elevated_view_grant` and route them
/// through [`GrantSet::from_store_rows`].
#[must_use]
pub fn active_grants_for_principal(_principal_id: &str) -> GrantSet {
    GrantSet::empty()
}
