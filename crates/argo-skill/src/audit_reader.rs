//! Narrow read surface over the audit log used by quality
//! recompute (Mission Claim 4 §3 #77).
//!
//! The aggregation in [`crate::QualityInputs::from_audit_history`]
//! only needs a tiny slice of an audit row (`kind`, `payload`,
//! `created_at`). Wrapping that slice in [`AuditRow`] + the
//! [`AuditStoreReader`] trait keeps the dependency on
//! `argo-audit` from leaking into the public `argo-skill` API and
//! lets unit tests fake the store with a `Vec<AuditRow>`.
//!
//! The concrete [`SessionStoreAuditReader`] adapter filters
//! `tool_result` audit rows by the active skill's `name` (skill
//! invocations surface as tool dispatches in the audit log today —
//! there is no dedicated `SkillExecuted` audit kind yet, see PR
//! body "Sensible deviations"). When that audit kind eventually
//! lands, the adapter can switch to `skill_id`-keyed filtering
//! without touching `from_audit_history`.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

/// Thin DTO of one audit row consumed by quality recompute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRow {
    /// Wire label, e.g. `tool_result`.
    pub kind: String,
    /// Canonical JSON payload as written to `audit_record.payload_json`.
    pub payload: serde_json::Value,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Narrow read surface the quality aggregator depends on. The
/// implementor decides how to filter `skill_id` (today: skill name
/// match against the audit `tool` payload field); the aggregator
/// only iterates the returned rows.
#[async_trait]
pub trait AuditStoreReader: Send + Sync {
    /// Audit rows that should contribute to `skill_id`'s quality
    /// inputs. Implementors MUST exclude rows older than `since`
    /// when it is `Some`. Returned rows do not need to be sorted.
    async fn rows_for_skill(
        &self,
        skill_id: &SkillId,
        since: Option<DateTime<Utc>>,
    ) -> ArgoResult<Vec<AuditRow>>;
}

/// Adapter from an audit-backed [`SessionStore`] to an
/// [`AuditStoreReader`]. Filters `tool_result` rows by
/// `payload.tool == skill_name`. The caller supplies the resolved
/// name at construction so this module stays free of a
/// `SkillStore` lookup (and the cycle that would introduce).
pub struct SessionStoreAuditReader {
    store: Arc<SessionStore>,
    skill_name: String,
}

impl SessionStoreAuditReader {
    /// Wrap a [`SessionStore`] with the skill's display name.
    #[must_use]
    pub fn new(store: Arc<SessionStore>, skill_name: String) -> Self {
        Self { store, skill_name }
    }
}

#[async_trait]
impl AuditStoreReader for SessionStoreAuditReader {
    async fn rows_for_skill(
        &self,
        _skill_id: &SkillId,
        since: Option<DateTime<Utc>>,
    ) -> ArgoResult<Vec<AuditRow>> {
        let since_micros = since.map_or(i64::MIN, |t| t.timestamp_micros());
        let rows = sqlx::query(
            "SELECT kind, payload_json, created_at FROM audit_record \
             WHERE kind = 'tool_result' AND created_at >= ?",
        )
        .bind(since_micros)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("audit history read: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let kind: String = row.get("kind");
            let payload_json: String = row.get("payload_json");
            let created_at_micros: i64 = row.get("created_at");
            let Ok(payload) = serde_json::from_str::<serde_json::Value>(&payload_json) else {
                continue;
            };
            if payload.get("tool").and_then(|t| t.as_str()) != Some(self.skill_name.as_str()) {
                continue;
            }
            let Some(created_at) = DateTime::<Utc>::from_timestamp_micros(created_at_micros)
            else {
                continue;
            };
            out.push(AuditRow {
                kind,
                payload,
                created_at,
            });
        }
        Ok(out)
    }
}
