//! `RedactionPolicy` plus the helpers that resolve it from a viewer's
//! permissions and active elevated-view grants.
//!
//! v1 keeps the policy struct dumb-simple (one [`PolicyMode`] per
//! category, no priority / chain-of-overrides). When NER + admin-gated
//! grant flows arrive (Track H), the resolver is the seam that learns
//! to fold extra signals.

use serde::{Deserialize, Serialize};

use crate::grants::GrantSet;
use crate::patterns::{Category, ALL_CATEGORIES};

/// Per-category masking decision. `Mask` writes a sentinel, `Show`
/// leaves the raw text in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    /// Replace every regex hit with the category's sentinel.
    Mask,
    /// Leave the raw text in place (caller has been granted the
    /// `unredacted_<category>` permission or has an active grant).
    Show,
}

/// One [`PolicyMode`] per recognised category. Default constructor
/// returns the strict (mask-everything) shape — the gateway uses that
/// for anonymous and default-permission principals.
///
/// ```
/// use argo_redact::{RedactionPolicy, PolicyMode};
/// let strict = RedactionPolicy::default();
/// assert!(matches!(strict.email, PolicyMode::Mask));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionPolicy {
    /// Mode for email matches.
    pub email: PolicyMode,
    /// Mode for phone matches.
    pub phone: PolicyMode,
    /// Mode for Luhn-valid card matches.
    pub card: PolicyMode,
    /// Mode for IBAN matches.
    pub iban: PolicyMode,
    /// Mode for NZ IRD matches.
    pub ird: PolicyMode,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self {
            email: PolicyMode::Mask,
            phone: PolicyMode::Mask,
            card: PolicyMode::Mask,
            iban: PolicyMode::Mask,
            ird: PolicyMode::Mask,
        }
    }
}

impl RedactionPolicy {
    /// Return the policy mode for a given category. Used by the
    /// redactor when iterating matches.
    #[must_use]
    pub fn mode(&self, category: Category) -> PolicyMode {
        match category {
            Category::Email => self.email,
            Category::Phone => self.phone,
            Category::Card => self.card,
            Category::Iban => self.iban,
            Category::Ird => self.ird,
        }
    }

    /// Build a policy where every category is shown. Used by elevated
    /// `unredacted-view` flows and tests.
    #[must_use]
    pub fn show_all() -> Self {
        Self {
            email: PolicyMode::Show,
            phone: PolicyMode::Show,
            card: PolicyMode::Show,
            iban: PolicyMode::Show,
            ird: PolicyMode::Show,
        }
    }
}

/// Per-viewer permission lookup, populated by the caller before the
/// redaction pass runs. Mirrors the `permissions.unredacted_*` shape
/// referenced in the brief — the gateway is the bridge that loads
/// these from the `Principal`'s RBAC permissions table.
///
/// The struct is intentionally narrow: only the five v1 categories
/// are addressable. New categories require both a new [`Category`]
/// variant and a new field here, so the compiler flags any drift.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerPermissions {
    /// `permissions.unredacted_email = true` → category shown.
    pub unredacted_email: bool,
    /// `permissions.unredacted_phone = true` → category shown.
    pub unredacted_phone: bool,
    /// `permissions.unredacted_card = true` → category shown.
    pub unredacted_card: bool,
    /// `permissions.unredacted_iban = true` → category shown.
    pub unredacted_iban: bool,
    /// `permissions.unredacted_ird = true` → category shown.
    pub unredacted_ird: bool,
}

impl ViewerPermissions {
    /// Strict-by-default constructor: every category masked.
    #[must_use]
    pub fn strict() -> Self {
        Self::default()
    }

    /// All-categories-unredacted helper for tests + the temporary
    /// `unredacted-view` admin path.
    #[must_use]
    pub fn allow_all() -> Self {
        Self {
            unredacted_email: true,
            unredacted_phone: true,
            unredacted_card: true,
            unredacted_iban: true,
            unredacted_ird: true,
        }
    }

    fn allows(self, category: Category) -> bool {
        match category {
            Category::Email => self.unredacted_email,
            Category::Phone => self.unredacted_phone,
            Category::Card => self.unredacted_card,
            Category::Iban => self.unredacted_iban,
            Category::Ird => self.unredacted_ird,
        }
    }
}

/// Resolve a [`RedactionPolicy`] for `viewer` given any active grants.
/// A category resolves to `Show` if EITHER the viewer's permission
/// flag is true OR an active grant covers it; otherwise `Mask`.
///
/// Expired grants must already be filtered out by `GrantSet::active`
/// before this is called — the resolver treats every grant in the
/// set as active.
///
/// ```
/// use argo_redact::{resolve_policy, GrantSet, PolicyMode, ViewerPermissions};
/// let p = resolve_policy(&ViewerPermissions { unredacted_email: true, ..Default::default() }, &GrantSet::empty());
/// assert!(matches!(p.email, PolicyMode::Show));
/// assert!(matches!(p.phone, PolicyMode::Mask));
/// ```
#[must_use]
pub fn resolve_policy(viewer: &ViewerPermissions, grants: &GrantSet) -> RedactionPolicy {
    let mut p = RedactionPolicy::default();
    for cat in ALL_CATEGORIES {
        let granted = viewer.allows(cat) || grants.covers(cat);
        let mode = if granted {
            PolicyMode::Show
        } else {
            PolicyMode::Mask
        };
        match cat {
            Category::Email => p.email = mode,
            Category::Phone => p.phone = mode,
            Category::Card => p.card = mode,
            Category::Iban => p.iban = mode,
            Category::Ird => p.ird = mode,
        }
    }
    p
}
