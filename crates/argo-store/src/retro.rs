//! Per-worker retrospective store (Stream `mission-retro-review-ui`,
//! MISSION_BACKLOG Section 6 #209).
//!
//! After a worker completes a batch of actions (or a long-running
//! duty wraps), the operator runs a retro over a time window: the
//! `metrics_json` column carries counts aggregated from `audit_record`
//! frozen at creation, and `sections_json` is the editable 5-section
//! template (Surface metrics / What went well / Surprises / What to
//! change / Action items). `finalized_at` is non-null once the
//! operator locks the row — further `update_sections` calls return
//! [`RetroError::Finalized`].
//!
//! Schema is owned by `migrations/v60.rs`; callers must run
//! `SessionStore::migrate` before constructing a [`RetroStore`].
//!
//! The metric aggregator scopes to one `worker_id` by joining
//! `audit_record.session_id` against `session.worker_id`. Audit kinds
//! recognised today:
//!
//! - `tool_call` → `actions_total`
//! - `approval.granted` → `actions_approved`
//! - `approval.revoked` → `actions_refused`
//! - `escalation.required` → `escalations`
//! - `skill.approved` → `learned_skills_added`
//! - `halt` → `error_count`
//!
//! Unknown kinds are ignored (forward-compat with future
//! `argo_audit::AuditKind` variants).
use crate::StoreResult;
use argo_types::ArgoError;
use serde_json::{json, Value};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// Default time-window depth used by the gateway when the caller
/// doesn't pass an explicit `window_start` / `window_end` pair. Seven
/// days mirrors the brief's "last 7 days" default.
pub const DEFAULT_WINDOW_DAYS: i64 = 7;

/// One persisted retro row.
#[derive(Debug, Clone)]
pub struct RetroRow {
    /// Retro identifier (UUIDv7 — created_at is encoded in the
    /// timestamp prefix).
    pub id: Uuid,
    /// Tenant this retro belongs to.
    pub tenant_id: Vec<u8>,
    /// Worker the retro covers.
    pub worker_id: Vec<u8>,
    /// Inclusive window start (unix seconds, UTC).
    pub window_start: i64,
    /// Inclusive window end (unix seconds, UTC).
    pub window_end: i64,
    /// Frozen metrics aggregated from `audit_record` at creation
    /// time. Shape is [`metrics_template`]'s output for any
    /// `worker_id` with no audit rows.
    pub metrics_json: Value,
    /// Editable 5-section operator template. Shape is
    /// [`section_template`]'s output until the operator fills sections in.
    pub sections_json: Value,
    /// When the row was inserted (unix seconds, UTC).
    pub created_at: i64,
    /// When the retro was locked. `None` while the draft is still
    /// editable.
    pub finalized_at: Option<i64>,
}

impl RetroRow {
    /// `true` when the operator has finalised the retro — the gateway
    /// PATCH handler rejects mutations against finalised rows.
    #[must_use]
    pub fn is_finalized(&self) -> bool {
        self.finalized_at.is_some()
    }
}

/// Error type for [`RetroStore`] mutations. Distinguishes the two
/// "expected" failure modes the gateway translates to specific HTTP
/// status codes (404 for missing, 409 for finalised).
#[derive(Debug, thiserror::Error)]
pub enum RetroError {
    /// No retro row matched the requested id.
    #[error("retro not found")]
    NotFound,
    /// Attempted to mutate a finalised retro.
    #[error("retro is finalized")]
    Finalized,
    /// Underlying SQL / serialization failure. Surfaced through the
    /// `ArgoError::Provider` envelope so the gateway returns 500.
    #[error("{0}")]
    Backend(#[from] ArgoError),
}

impl From<sqlx::Error> for RetroError {
    fn from(e: sqlx::Error) -> Self {
        Self::Backend(ArgoError::Provider(format!("retro: {e}")))
    }
}

/// New-retro payload — the [`RetroStore::create`] caller fills the
/// fields the gateway derives (tenant, worker, window) and the store
/// records the metrics + a fresh section template.
pub struct NewRetro {
    /// Tenant the retro belongs to.
    pub tenant_id: Vec<u8>,
    /// Worker the retro covers.
    pub worker_id: Vec<u8>,
    /// Inclusive window start (unix seconds, UTC).
    pub window_start: i64,
    /// Inclusive window end (unix seconds, UTC).
    pub window_end: i64,
    /// Frozen metrics aggregated by the caller; the store does not
    /// recompute. The gateway calls [`aggregate_metrics`] before
    /// invoking `create`.
    pub metrics_json: Value,
    /// Creation timestamp (unix seconds, UTC).
    pub created_at: i64,
}

/// Wire-shape sqlx returns for a single `retro` row. Hoisted into
/// a type alias because clippy's `type_complexity` lint reports
/// 9-tuples otherwise.
type RetroSqlRow = (
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    i64,
    i64,
    String,
    String,
    i64,
    Option<i64>,
);

/// Thin wrapper around the workspace SQLite pool.
pub struct RetroStore<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> RetroStore<'a> {
    /// Construct over an already-migrated workspace pool.
    #[must_use]
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Insert a new retro row, returning the generated id. The
    /// `sections_json` column is initialised to the empty 5-section
    /// template — operators fill it in via subsequent PATCH calls.
    ///
    /// # Errors
    ///
    /// Returns [`RetroError::Backend`] on SQL execution failure.
    pub async fn create(&self, new: NewRetro) -> Result<Uuid, RetroError> {
        let id = Uuid::now_v7();
        let sections = serde_json::to_string(&section_template()).map_err(|e| {
            RetroError::Backend(ArgoError::Provider(format!("retro section template: {e}")))
        })?;
        let metrics = serde_json::to_string(&new.metrics_json).map_err(|e| {
            RetroError::Backend(ArgoError::Provider(format!("retro metrics encode: {e}")))
        })?;
        sqlx::query(
            "INSERT INTO retro (id, tenant_id, worker_id, window_start, window_end, \
             metrics_json, sections_json, created_at, finalized_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(new.tenant_id)
        .bind(new.worker_id)
        .bind(new.window_start)
        .bind(new.window_end)
        .bind(&metrics)
        .bind(&sections)
        .bind(new.created_at)
        .execute(self.pool)
        .await?;
        Ok(id)
    }

    /// Fetch a retro by id. Returns [`RetroError::NotFound`] when no
    /// row matches.
    pub async fn get(&self, id: Uuid) -> Result<RetroRow, RetroError> {
        let row: Option<RetroSqlRow> = sqlx::query_as(
            "SELECT id, tenant_id, worker_id, window_start, window_end, \
             metrics_json, sections_json, created_at, finalized_at \
             FROM retro WHERE id = ?",
        )
        .bind(id.as_bytes().as_slice())
        .fetch_optional(self.pool)
        .await?;
        let row = row.ok_or(RetroError::NotFound)?;
        decode_row(row)
    }

    /// List retros for one worker, newest first.
    pub async fn list_for_worker(
        &self,
        worker_id: &[u8],
        limit: i64,
    ) -> Result<Vec<RetroRow>, RetroError> {
        let rows: Vec<RetroSqlRow> = sqlx::query_as(
            "SELECT id, tenant_id, worker_id, window_start, window_end, \
             metrics_json, sections_json, created_at, finalized_at \
             FROM retro WHERE worker_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(worker_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        rows.into_iter().map(decode_row).collect()
    }

    /// List every retro in the tenant, newest first. Used by the
    /// `/retro` index page; theme search runs client-side over the
    /// section text. `q` is a SQL `LIKE` substring on the sections
    /// JSON column — case-insensitive, no FTS5 (the workspace's FTS5
    /// table is wired to messages, not retro sections).
    pub async fn list_all(
        &self,
        tenant_id: &[u8],
        q: Option<&str>,
        limit: i64,
    ) -> Result<Vec<RetroRow>, RetroError> {
        let rows: Vec<RetroSqlRow> =
            if let Some(needle) = q.map(str::trim).filter(|s| !s.is_empty()) {
                let pattern = format!("%{}%", needle.to_lowercase());
                sqlx::query_as(
                    "SELECT id, tenant_id, worker_id, window_start, window_end, \
                     metrics_json, sections_json, created_at, finalized_at \
                     FROM retro \
                     WHERE tenant_id = ? AND LOWER(sections_json) LIKE ? \
                     ORDER BY created_at DESC LIMIT ?",
                )
                .bind(tenant_id)
                .bind(pattern)
                .bind(limit)
                .fetch_all(self.pool)
                .await?
            } else {
                sqlx::query_as(
                    "SELECT id, tenant_id, worker_id, window_start, window_end, \
                     metrics_json, sections_json, created_at, finalized_at \
                     FROM retro WHERE tenant_id = ? ORDER BY created_at DESC LIMIT ?",
                )
                .bind(tenant_id)
                .bind(limit)
                .fetch_all(self.pool)
                .await?
            };
        rows.into_iter().map(decode_row).collect()
    }

    /// Overwrite `sections_json` on a draft retro. Returns
    /// [`RetroError::NotFound`] when the id is unknown, or
    /// [`RetroError::Finalized`] when the row is locked.
    pub async fn update_sections(
        &self,
        id: Uuid,
        sections: &Value,
    ) -> Result<(), RetroError> {
        let current = self.get(id).await?;
        if current.is_finalized() {
            return Err(RetroError::Finalized);
        }
        let encoded = serde_json::to_string(sections).map_err(|e| {
            RetroError::Backend(ArgoError::Provider(format!("sections encode: {e}")))
        })?;
        sqlx::query("UPDATE retro SET sections_json = ? WHERE id = ?")
            .bind(encoded)
            .bind(id.as_bytes().as_slice())
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Stamp `finalized_at` on a draft retro. Idempotent: re-calling
    /// `finalize` on an already-finalised row returns
    /// [`RetroError::Finalized`] so the gateway can return 409 and
    /// the operator notices the no-op.
    pub async fn finalize(&self, id: Uuid, now_unix: i64) -> Result<(), RetroError> {
        let current = self.get(id).await?;
        if current.is_finalized() {
            return Err(RetroError::Finalized);
        }
        sqlx::query("UPDATE retro SET finalized_at = ? WHERE id = ?")
            .bind(now_unix)
            .bind(id.as_bytes().as_slice())
            .execute(self.pool)
            .await?;
        Ok(())
    }
}

/// Aggregate audit-row counts for one worker across `[start, end]`.
/// Joins `audit_record` against `session.worker_id` so the resulting
/// counts are worker-scoped even when many sessions share the worker.
///
/// Caller-visible keys match the brief's metric vocabulary;
/// `error_count_by_category` is a JSON object so future audit-kind
/// expansions can land additional categories without a wire-shape
/// migration.
pub async fn aggregate_metrics(
    pool: &Pool<Sqlite>,
    worker_id: &[u8],
    start_unix: i64,
    end_unix: i64,
) -> StoreResult<Value> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT a.kind, COUNT(*) FROM audit_record a \
         JOIN session s ON a.session_id = s.id \
         WHERE s.worker_id = ? \
           AND a.created_at >= ? AND a.created_at <= ? \
         GROUP BY a.kind",
    )
    .bind(worker_id)
    .bind(start_unix)
    .bind(end_unix)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("aggregate_metrics: {e}")))?;

    let mut actions_total: i64 = 0;
    let mut actions_approved: i64 = 0;
    let mut actions_refused: i64 = 0;
    let mut escalations: i64 = 0;
    let mut learned_skills_added: i64 = 0;
    let mut error_by_category = serde_json::Map::new();

    for (kind, count) in rows {
        match kind.as_str() {
            "tool_call" => actions_total += count,
            "approval.granted" => actions_approved += count,
            "approval.revoked" => actions_refused += count,
            "escalation.required" => escalations += count,
            "skill.approved" => learned_skills_added += count,
            "halt" => {
                error_by_category.insert("halt".to_string(), json!(count));
            }
            // Forward-compat: any new audit kind we don't recognise
            // contributes to the generic "other" bucket so the
            // operator notices unexpected activity in the metrics
            // panel without us rewriting the aggregator.
            other if other.starts_with("rbac.")
                || other.contains(".rejected")
                || other == "lock.violate" =>
            {
                let key = other.to_string();
                error_by_category
                    .entry(key)
                    .and_modify(|v| {
                        let prev = v.as_i64().unwrap_or(0);
                        *v = json!(prev + count);
                    })
                    .or_insert(json!(count));
            }
            _ => {}
        }
    }

    Ok(json!({
        "actions_total": actions_total,
        "actions_approved": actions_approved,
        "actions_refused": actions_refused,
        "escalations": escalations,
        "learned_skills_added": learned_skills_added,
        "error_count_by_category": Value::Object(error_by_category),
    }))
}

/// Empty-metrics scaffold — useful when the caller knows the worker
/// has no audit history yet but still wants to record a retro (e.g.
/// the operator is making forward-looking notes).
#[must_use]
pub fn metrics_template() -> Value {
    json!({
        "actions_total": 0,
        "actions_approved": 0,
        "actions_refused": 0,
        "escalations": 0,
        "learned_skills_added": 0,
        "error_count_by_category": {},
    })
}

/// Fresh 5-section operator-fillable template. The wire keys are the
/// load-bearing shape the frontend dialog binds to; renames here
/// require a UI migration.
#[must_use]
pub fn section_template() -> Value {
    json!({
        "surface_metrics": "",
        "what_went_well": "",
        "surprises": "",
        "what_to_change": "",
        "action_items": "",
    })
}

fn decode_row(row: RetroSqlRow) -> Result<RetroRow, RetroError> {
    let (
        id_blob,
        tenant_id,
        worker_id,
        window_start,
        window_end,
        metrics_str,
        sections_str,
        created_at,
        finalized_at,
    ) = row;
    let id = Uuid::from_slice(&id_blob).map_err(|e| {
        RetroError::Backend(ArgoError::Provider(format!("retro id decode: {e}")))
    })?;
    let metrics_json: Value = serde_json::from_str(&metrics_str).map_err(|e| {
        RetroError::Backend(ArgoError::Provider(format!("metrics decode: {e}")))
    })?;
    let sections_json: Value = serde_json::from_str(&sections_str).map_err(|e| {
        RetroError::Backend(ArgoError::Provider(format!("sections decode: {e}")))
    })?;
    Ok(RetroRow {
        id,
        tenant_id,
        worker_id,
        window_start,
        window_end,
        metrics_json,
        sections_json,
        created_at,
        finalized_at,
    })
}

#[cfg(test)]
#[path = "retro_tests.rs"]
mod tests;
