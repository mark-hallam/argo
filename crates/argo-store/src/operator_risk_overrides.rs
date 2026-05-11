//! Per-operator risk-level override store (Stream
//! `mission-operator-specific-risk-overrides`, MISSION_BACKLOG
//! Section 3 Claim 6 #122).
//!
//! The default policy classifies every action via the bound vertical's
//! `[actions.<name>].risk` declaration. An operator can replace that
//! default for one `(action_kind, vertical_id)` pair by upserting a
//! row here. Missing row → caller falls back to the manifest default.
//!
//! `vertical_id == ''` means "apply for every vertical this operator
//! touches" — per-vertical rows win over the all-verticals scope.
//!
//! Schema is owned by `migrations/v58.rs`; callers must run
//! `SessionStore::migrate` before constructing a
//! [`OperatorRiskOverridesStore`].
use crate::StoreResult;
use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};

/// One stored override row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorRiskOverrideRow {
    /// Operator the override belongs to.
    pub operator_id: String,
    /// Action kind whose risk classification is being overridden.
    pub action_kind: String,
    /// Vertical the override applies to. `None` means every vertical
    /// this operator touches (stored as `''` in SQL).
    pub vertical_id: Option<String>,
    /// Replacement risk level. One of `"low" | "medium" | "high"`.
    pub risk_level: String,
    /// When `true`, the override is allowed to lower the classification
    /// below the manifest default. When `false`, the resolver refuses
    /// downgrades.
    pub allow_downgrade: bool,
    /// Unix timestamp, seconds, when the row was last written.
    pub created_at: i64,
}

/// Thin wrapper around the workspace SQLite pool.
pub struct OperatorRiskOverridesStore<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> OperatorRiskOverridesStore<'a> {
    /// Construct over an already-migrated workspace pool.
    #[must_use]
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Upsert one operator's override for `(action_kind, vertical_id)`.
    /// Returns the `created_at` written so callers can echo it back.
    ///
    /// # Errors
    ///
    /// Returns [`ArgoError::Provider`] on SQL execution failure or when
    /// `risk_level` is not one of `"low" | "medium" | "high"`.
    pub async fn set(
        &self,
        operator_id: &str,
        action_kind: &str,
        vertical_id: Option<&str>,
        risk_level: &str,
        allow_downgrade: bool,
        now_unix: i64,
    ) -> StoreResult<i64> {
        validate_risk_level(risk_level)?;
        let vertical = vertical_id.unwrap_or("");
        let allow_int = i64::from(allow_downgrade);
        sqlx::query(
            "INSERT INTO operator_risk_overrides \
                 (operator_id, action_kind, vertical_id, risk_level, allow_downgrade, created_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT (operator_id, action_kind, vertical_id) DO UPDATE SET \
                 risk_level      = excluded.risk_level, \
                 allow_downgrade = excluded.allow_downgrade, \
                 created_at      = excluded.created_at",
        )
        .bind(operator_id)
        .bind(action_kind)
        .bind(vertical)
        .bind(risk_level)
        .bind(allow_int)
        .bind(now_unix)
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("operator_risk_overrides set: {e}")))?;
        Ok(now_unix)
    }

    /// Read every override row for one operator. Sorted by
    /// `(action_kind, vertical_id)` for stable CLI output.
    pub async fn get_for_operator(
        &self,
        operator_id: &str,
    ) -> StoreResult<Vec<OperatorRiskOverrideRow>> {
        let rows: Vec<(String, String, String, i64, i64)> = sqlx::query_as(
            "SELECT action_kind, vertical_id, risk_level, allow_downgrade, created_at \
             FROM operator_risk_overrides WHERE operator_id = ? \
             ORDER BY action_kind ASC, vertical_id ASC",
        )
        .bind(operator_id)
        .fetch_all(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("operator_risk_overrides get: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|(action_kind, vertical_id, risk_level, allow, created_at)| {
                OperatorRiskOverrideRow {
                    operator_id: operator_id.to_string(),
                    action_kind,
                    vertical_id: non_empty_vertical(vertical_id),
                    risk_level,
                    allow_downgrade: allow != 0,
                    created_at,
                }
            })
            .collect())
    }

    /// Read every override row across every operator. Sorted by
    /// `(operator_id, action_kind, vertical_id)` for stable output.
    pub async fn list(&self) -> StoreResult<Vec<OperatorRiskOverrideRow>> {
        let rows: Vec<(String, String, String, String, i64, i64)> = sqlx::query_as(
            "SELECT operator_id, action_kind, vertical_id, risk_level, allow_downgrade, \
                    created_at \
             FROM operator_risk_overrides \
             ORDER BY operator_id ASC, action_kind ASC, vertical_id ASC",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("operator_risk_overrides list: {e}")))?;
        Ok(rows
            .into_iter()
            .map(
                |(operator_id, action_kind, vertical_id, risk_level, allow, created_at)| {
                    OperatorRiskOverrideRow {
                        operator_id,
                        action_kind,
                        vertical_id: non_empty_vertical(vertical_id),
                        risk_level,
                        allow_downgrade: allow != 0,
                        created_at,
                    }
                },
            )
            .collect())
    }

    /// Delete one override row keyed on
    /// `(operator_id, action_kind, vertical_id)`. Returns `true` when
    /// a row was removed so callers can distinguish "cleared" from
    /// "no row".
    pub async fn delete(
        &self,
        operator_id: &str,
        action_kind: &str,
        vertical_id: Option<&str>,
    ) -> StoreResult<bool> {
        let vertical = vertical_id.unwrap_or("");
        let result = sqlx::query(
            "DELETE FROM operator_risk_overrides \
             WHERE operator_id = ? AND action_kind = ? AND vertical_id = ?",
        )
        .bind(operator_id)
        .bind(action_kind)
        .bind(vertical)
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("operator_risk_overrides delete: {e}")))?;
        Ok(result.rows_affected() > 0)
    }
}

fn validate_risk_level(level: &str) -> StoreResult<()> {
    match level {
        "low" | "medium" | "high" => Ok(()),
        other => Err(ArgoError::Provider(format!(
            "operator_risk_overrides set: risk_level {other:?} \
             must be one of low / medium / high"
        ))),
    }
}

fn non_empty_vertical(v: String) -> Option<String> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

#[cfg(test)]
#[path = "operator_risk_overrides_tests.rs"]
mod tests;
