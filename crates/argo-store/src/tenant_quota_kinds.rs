//! Quota kind enums + error type for tenant_quota gating.
//!
//! Split out of `tenant_quota.rs` for the 500-LOC budget. The
//! storage-layer methods that consume these types live in
//! `tenant_quota.rs`.

use crate::tenant_quota::TenantQuotaSummary;
use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// Resource dimension checked by
/// [`crate::tenant_quota::TenantQuotaStore::has_headroom_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantQuotaKind {
    /// USD micros against `max_usd_micros`.
    UsdMicros,
    /// Action count against `max_actions`.
    Actions,
    /// Worker count against `max_workers`.
    Workers,
}

/// Hard-enforced action kind for the gateway quota gate. Each
/// variant maps to one of the existing tenant-quota caps; the
/// label survives into the API error body so callers know which
/// dimension was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaActionKind {
    /// `session.create` — gated against `max_workers` /
    /// `used_workers`.
    Sessions,
    /// `message.add` — gated against `max_actions` /
    /// `used_actions`.
    Messages,
    /// `forge.run` — gated against `max_actions` /
    /// `used_actions`.
    ForgeRuns,
    /// `skill.execute` — gated against `max_actions` /
    /// `used_actions`.
    SkillInvocations,
}

impl QuotaActionKind {
    /// Stable kind label used in JSON responses and logs.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_store::tenant_quota_kinds::QuotaActionKind;
    /// assert_eq!(QuotaActionKind::Sessions.as_str(), "sessions");
    /// ```
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sessions => "sessions",
            Self::Messages => "messages",
            Self::ForgeRuns => "forge_runs",
            Self::SkillInvocations => "skill_invocations",
        }
    }

    /// Resolve `(current, limit)` from a tenant summary. Sessions
    /// gate against worker headroom; the other three share the
    /// action counter (the schema has only one action cap).
    pub(crate) fn usage_pair(self, summary: &TenantQuotaSummary) -> (u64, Option<u64>) {
        match self {
            Self::Sessions => (
                summary.usage.used_workers,
                summary.quota.as_ref().and_then(|q| q.max_workers),
            ),
            Self::Messages | Self::ForgeRuns | Self::SkillInvocations => (
                summary.usage.used_actions,
                summary.quota.as_ref().and_then(|q| q.max_actions),
            ),
        }
    }
}

impl fmt::Display for QuotaActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for QuotaActionKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sessions" => Ok(Self::Sessions),
            "messages" => Ok(Self::Messages),
            "forge_runs" => Ok(Self::ForgeRuns),
            "skill_invocations" => Ok(Self::SkillInvocations),
            other => Err(format!("unknown QuotaActionKind '{other}'")),
        }
    }
}

/// Error returned by
/// [`crate::tenant_quota::TenantQuotaStore::check_or_reject`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaError {
    /// Adding `delta` would push usage over the configured cap.
    Exceeded {
        /// Kind label (snake_case).
        kind: String,
        /// Current usage at check time.
        current: u64,
        /// Configured cap.
        limit: u64,
    },
    /// Underlying store error during the read-side of the check.
    Internal(String),
}

impl fmt::Display for QuotaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exceeded {
                kind,
                current,
                limit,
            } => write!(
                f,
                "quota_exceeded: kind={kind} current={current} limit={limit}"
            ),
            Self::Internal(msg) => write!(f, "quota_internal: {msg}"),
        }
    }
}

impl std::error::Error for QuotaError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_kind_round_trip() {
        for kind in [
            QuotaActionKind::Sessions,
            QuotaActionKind::Messages,
            QuotaActionKind::ForgeRuns,
            QuotaActionKind::SkillInvocations,
        ] {
            assert_eq!(QuotaActionKind::from_str(kind.as_str()), Ok(kind));
            assert_eq!(kind.to_string(), kind.as_str());
        }
        assert!(QuotaActionKind::from_str("nope").is_err());
    }

    #[test]
    fn quota_error_displays() {
        let e = QuotaError::Exceeded {
            kind: "messages".to_string(),
            current: 5,
            limit: 5,
        };
        let s = e.to_string();
        assert!(s.contains("messages"));
        assert!(s.contains("current=5"));
        assert!(s.contains("limit=5"));
    }
}
