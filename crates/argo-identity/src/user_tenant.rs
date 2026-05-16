//! User ↔ tenant membership link with a per-tenant role
//! (Stream `mission-tenant-substrate-tables`).
//!
//! One row per `(user_id, tenant_id)` pair; a user may hold different
//! roles in different tenants. Read paths consume this table to answer
//! "what tenants can Alice see?" and "who are the owners of tenant X?".
//!
//! ```
//! use argo_identity::tenant::Tenant;
//! use argo_identity::user::UserAccount;
//! use argo_identity::user_tenant::{UserTenant, UserTenantRole};
//! let tenant = Tenant::new("Acme".into(), "billing@acme.com".into());
//! let user = UserAccount::new("alice@acme.com".into());
//! let link = UserTenant::new(user.id, tenant.id, UserTenantRole::Owner);
//! assert_eq!(link.role, UserTenantRole::Owner);
//! ```

use crate::user::UserId;
use argo_types::TenantId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// One row of the `user_tenant` join table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserTenant {
    /// Linked user.
    pub user_id: UserId,
    /// Linked tenant.
    pub tenant_id: TenantId,
    /// Role the user holds within this tenant.
    pub role: UserTenantRole,
    /// Unix-seconds timestamp of insert.
    pub created_at: i64,
}

impl UserTenant {
    /// Build a fresh link row with `created_at = now()`.
    #[must_use]
    pub fn new(user_id: UserId, tenant_id: TenantId, role: UserTenantRole) -> Self {
        Self {
            user_id,
            tenant_id,
            role,
            created_at: Utc::now().timestamp(),
        }
    }
}

/// Role a user holds within a single tenant. Storage is case-sensitive
/// lowercase (`'owner' | 'admin' | 'user' | 'guest'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserTenantRole {
    /// Full billing + delete authority. Only owners can delete a tenant.
    Owner,
    /// Operator-level access without billing authority.
    Admin,
    /// Standard authenticated user — runs workers, no admin surfaces.
    User,
    /// Read-mostly visitor; one-off shared-link recipients.
    Guest,
}

impl UserTenantRole {
    /// Stable lowercase label used for SQL storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::User => "user",
            Self::Guest => "guest",
        }
    }
}

impl FromStr for UserTenantRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            "guest" => Ok(Self::Guest),
            other => Err(format!("unknown user_tenant role '{other}'")),
        }
    }
}
