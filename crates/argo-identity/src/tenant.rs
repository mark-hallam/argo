//! Row-backed tenant metadata for the operator surface
//! (Stream `mission-tenant-substrate-tables`).
//!
//! Tenant identity remains the [`TenantId`] UUID plumbed through every
//! primitive (CLAUDE.md invariant #2). This type carries the *metadata*
//! a freshly-provisioned tenant needs — operator-facing name, billing
//! email, lifecycle status — so the self-serve signup, audit, and
//! billing surfaces can enumerate tenants instead of looking them up by
//! UUID. Existing `workspace`, `tenant_quota`, `tenant_rate_limit`,
//! `tenant_tier`, and `tenant_usage` rows are *not* re-anchored: this
//! table is metadata, not authority.
//!
//! ```
//! use argo_identity::tenant::{Tenant, TenantStatus};
//! let t = Tenant::new("Acme".to_string(), "billing@acme.com".to_string());
//! assert_eq!(t.status, TenantStatus::Active);
//! assert_eq!(t.name, "Acme");
//! ```

use argo_types::TenantId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// One row of the `tenant` table — operator-facing metadata anchored
/// by [`TenantId`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tenant {
    /// Tenant identity. Same UUID plumbed through every primitive.
    pub id: TenantId,
    /// Operator-facing display name (e.g. `"Acme Co"`).
    pub name: String,
    /// Address where invoices and account alerts are delivered.
    pub billing_email: String,
    /// Lifecycle status — `Active`, `Suspended`, or `Deleted` (soft).
    pub status: TenantStatus,
    /// Unix-seconds timestamp of insert.
    pub created_at: i64,
    /// Unix-seconds timestamp of the most recent mutation.
    pub updated_at: i64,
}

impl Tenant {
    /// Construct a fresh `Tenant` with a generated [`TenantId`],
    /// `status = Active`, and `created_at`/`updated_at` set to `now()`.
    ///
    /// ```
    /// use argo_identity::tenant::Tenant;
    /// let t = Tenant::new("Acme".to_string(), "billing@acme.com".to_string());
    /// assert_eq!(t.created_at, t.updated_at);
    /// ```
    #[must_use]
    pub fn new(name: String, billing_email: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: TenantId::new(),
            name,
            billing_email,
            status: TenantStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Replace `status` and bump `updated_at` to `now()`.
    pub fn set_status(&mut self, status: TenantStatus) {
        self.status = status;
        self.updated_at = Utc::now().timestamp();
    }
}

/// Lifecycle state of a [`Tenant`]. Storage is case-sensitive lowercase
/// (`'active' | 'suspended' | 'deleted'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    /// Default state — billing active, all surfaces available.
    Active,
    /// Billing failure or operator hold — reads OK, writes refused.
    Suspended,
    /// Soft delete — audit + data-classification rows persist.
    Deleted,
}

impl TenantStatus {
    /// Stable lowercase label used for SQL storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Deleted => "deleted",
        }
    }
}

impl FromStr for TenantStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "deleted" => Ok(Self::Deleted),
            other => Err(format!("unknown tenant status '{other}'")),
        }
    }
}

#[cfg(test)]
#[path = "tenant_tests.rs"]
mod tests;
