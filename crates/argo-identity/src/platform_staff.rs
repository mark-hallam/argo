//! Platform-staff identity — Skycot's own operators (DEC-057 Phase 1).
//!
//! A platform-staff principal lives OUTSIDE any tenant and has NO standing
//! access to tenant data. It exists to run *account-level* operations — read
//! account status across tenants, suspend/reactivate a tenant — with every
//! action audited. It is deliberately a different *kind* of identity from a
//! tenant member: a [`crate::user_tenant::UserTenantRole`] (owner/admin/…)
//! never confers platform-staff power, and staff is **default-deny** — the
//! absence of a `platform_staff` row means "not staff".
//!
//! Phase 1 grants are account-level only. None of these roles can read a
//! tenant's *workspace contents* (messages, audit bodies, documents); that
//! consent-based in-workspace access is DEC-057 Phase 2.
//!
//! ```
//! use argo_identity::platform_staff::{PlatformRole, PlatformStaff, StaffAction};
//! use argo_identity::user::UserId;
//! let s = PlatformStaff::new(UserId::new(), PlatformRole::Support, None);
//! assert!(s.is_active());
//! // Support can read accounts but not suspend.
//! assert!(StaffAction::ReadAccounts.permitted(s.role));
//! assert!(!StaffAction::SuspendTenant.permitted(s.role));
//! ```

use crate::user::UserId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// One row of the `platform_staff` table — a Skycot operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformStaff {
    /// The staff member's user account (same `UserId` as their login).
    pub user_id: UserId,
    /// Account-level authority this operator holds.
    pub role: PlatformRole,
    /// Lifecycle status — a `Disabled` row never authorizes.
    pub status: PlatformStaffStatus,
    /// Unix-seconds timestamp the staff grant was created.
    pub created_at: i64,
    /// Principal that granted staff (audit attribution); `None` when seeded
    /// by the system/bootstrap CLI.
    pub created_by: Option<String>,
}

impl PlatformStaff {
    /// Build a fresh active staff row with `created_at = now()`.
    #[must_use]
    pub fn new(user_id: UserId, role: PlatformRole, created_by: Option<String>) -> Self {
        Self {
            user_id,
            role,
            status: PlatformStaffStatus::Active,
            created_at: Utc::now().timestamp(),
            created_by,
        }
    }

    /// True only when this staff grant is `Active`. Authorization checks
    /// MUST gate on this — a disabled grant authorizes nothing.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == PlatformStaffStatus::Active
    }
}

/// Account-level authority a platform-staff principal holds. Storage is
/// case-sensitive lowercase (`'support' | 'billing_ops' | 'super_admin'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformRole {
    /// Read account-level status across tenants. Cannot suspend/reactivate.
    Support,
    /// Account-level read + billing-driven suspend/reactivate.
    BillingOps,
    /// Full account-level authority (read + suspend/reactivate any tenant).
    SuperAdmin,
}

impl PlatformRole {
    /// Stable lowercase label used for SQL storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Support => "support",
            Self::BillingOps => "billing_ops",
            Self::SuperAdmin => "super_admin",
        }
    }

    /// True when this role may suspend/reactivate a tenant account.
    /// Read-only `Support` may not.
    #[must_use]
    pub fn can_suspend(self) -> bool {
        matches!(self, Self::BillingOps | Self::SuperAdmin)
    }
}

impl FromStr for PlatformRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "support" => Ok(Self::Support),
            "billing_ops" => Ok(Self::BillingOps),
            "super_admin" => Ok(Self::SuperAdmin),
            other => Err(format!("unknown platform-staff role '{other}'")),
        }
    }
}

/// Lifecycle state of a [`PlatformStaff`] grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformStaffStatus {
    /// Default — the grant authorizes per its role.
    Active,
    /// Revoked — sign-in still works as a normal user, but staff
    /// authority is withdrawn.
    Disabled,
}

impl PlatformStaffStatus {
    /// Stable lowercase label used for SQL storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }
}

impl FromStr for PlatformStaffStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            other => Err(format!("unknown platform-staff status '{other}'")),
        }
    }
}

/// A privileged platform-staff operation, used by the gateway gate to decide
/// whether a given [`PlatformRole`] is sufficient. Keeping the action→floor
/// mapping in one enum keeps authorization decisions in a single place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaffAction {
    /// Read the cross-tenant account list (account-level fields only).
    ReadAccounts,
    /// Suspend or reactivate a tenant account.
    SuspendTenant,
}

impl StaffAction {
    /// Whether `role` is permitted to perform this action.
    #[must_use]
    pub fn permitted(self, role: PlatformRole) -> bool {
        match self {
            // Any active staff role may read account-level status.
            Self::ReadAccounts => true,
            // Only roles with suspend authority may flip lifecycle.
            Self::SuspendTenant => role.can_suspend(),
        }
    }
}

#[cfg(test)]
#[path = "platform_staff_tests.rs"]
mod tests;
