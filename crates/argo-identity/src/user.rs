//! Row-backed user account for the magic-link sign-in flow
//! (Stream `mission-tenant-substrate-tables`).
//!
//! `email` is the verified identity column — the gateway looks rows up
//! by email when minting a magic-link session, and the UNIQUE
//! constraint on `user_account.email` prevents two accounts from
//! claiming the same address.
//!
//! [`UserId`] is defined locally rather than in `argo-types` because
//! the brief's may-touch list scopes new identity primitives to
//! `argo-identity`; downstream consumers (tenant provisioning, magic
//! link verify, audit attribution) re-export through this module.
//!
//! ```
//! use argo_identity::user::{UserAccount, UserStatus};
//! let u = UserAccount::new("alice@acme.com".to_string());
//! assert_eq!(u.status, UserStatus::Active);
//! assert!(u.last_login_at.is_none());
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Stable identity for one [`UserAccount`] row.
///
/// Defined here rather than in `argo-types::ids` because new identity
/// primitives are scoped to `argo-identity` by the brief; if a future
/// brief widens the `argo-types` may-touch list this can move there and
/// be re-exported from this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    /// Generate a fresh UUIDv7 user id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// One row of the `user_account` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAccount {
    /// Unique user identity.
    pub id: UserId,
    /// Magic-link target. Unique across all accounts.
    pub email: String,
    /// Operator-facing display name; `None` until first verify.
    pub display_name: Option<String>,
    /// Unix-seconds timestamp of insert.
    pub created_at: i64,
    /// Unix-seconds timestamp of most recent successful verify;
    /// `None` for users who have never signed in.
    pub last_login_at: Option<i64>,
    /// Lifecycle state — `Active` or `Disabled`.
    pub status: UserStatus,
}

impl UserAccount {
    /// Construct a freshly-created account with a generated [`UserId`],
    /// `status = Active`, no display name, and no last-login timestamp.
    #[must_use]
    pub fn new(email: String) -> Self {
        Self {
            id: UserId::new(),
            email,
            display_name: None,
            created_at: Utc::now().timestamp(),
            last_login_at: None,
            status: UserStatus::Active,
        }
    }
}

/// Lifecycle state of a [`UserAccount`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    /// Default state — magic-link sign-in permitted.
    Active,
    /// Operator-disabled — sign-in refused, audit rows retained.
    Disabled,
}

impl UserStatus {
    /// Stable lowercase label used for SQL storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }
}

impl FromStr for UserStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            other => Err(format!("unknown user status '{other}'")),
        }
    }
}

#[cfg(test)]
#[path = "user_tests.rs"]
mod tests;
