//! SQLite-backed CRUD over the `role_duty` and `duty_fired_session`
//! tables.
//!
//! Extracted from `duty_scheduler.rs` (mvp-split-duty-scheduler);
//! re-exported under that module for backward compatibility.
use crate::duty_scheduler::DutyRecord;
use crate::duty_scheduler_gate::{FireDecision, GateConfig};
use crate::SessionId;
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

const ROLE_DUTY_SCHEMA: &str =
    include_str!("../../argo-store/migrations/28__role_duties.sql");

/// SQLite-backed CRUD over the `role_duty` and `duty_fired_session`
/// tables.
pub struct DutyStore {
    store: Arc<SessionStore>,
}

impl DutyStore {
    /// Build over an open, migrated [`SessionStore`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Idempotently create the `role_duty` + `duty_fired_session`
    /// tables.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in ROLE_DUTY_SCHEMA
            .split_terminator(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            sqlx::query(stmt)
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("role_duty schema: {e}")))?;
        }
        Ok(())
    }

    /// Insert a fresh duty.
    pub async fn insert(&self, duty: &DutyRecord) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query(
            "INSERT INTO role_duty (id, tenant_id, worker_id, cron, entry_action, \
             description, enabled, paused_until, last_fired_at, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&duty.id)
        .bind(duty.tenant_id.0.as_bytes().as_slice())
        .bind(duty.worker_id.to_string())
        .bind(&duty.cron)
        .bind(&duty.entry_action)
        .bind(&duty.description)
        .bind(i64::from(duty.enabled))
        .bind(duty.paused_until.map(|d| d.timestamp_micros()))
        .bind(duty.last_fired_at.map(|d| d.timestamp_micros()))
        .bind(duty.created_at.timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert duty: {e}")))?;
        Ok(())
    }

    /// Look up one duty by id.
    pub async fn get(&self, id: &str) -> ArgoResult<Option<DutyRecord>> {
        self.ensure_schema().await?;
        let row = sqlx::query(
            "SELECT id, tenant_id, worker_id, cron, entry_action, description, \
             enabled, paused_until, last_fired_at, created_at \
             FROM role_duty WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("find duty: {e}")))?;
        row.map(row_to_duty).transpose()
    }

    /// List duties scoped to a tenant, oldest first.
    pub async fn list_for_tenant(&self, tenant: TenantId) -> ArgoResult<Vec<DutyRecord>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT id, tenant_id, worker_id, cron, entry_action, description, \
             enabled, paused_until, last_fired_at, created_at \
             FROM role_duty WHERE tenant_id = ? ORDER BY created_at ASC, id ASC",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list duties: {e}")))?;
        rows.into_iter().map(row_to_duty).collect()
    }

    /// List all enabled, non-paused duties across every tenant —
    /// the per-tick scheduler input.
    pub async fn list_active(&self, now: DateTime<Utc>) -> ArgoResult<Vec<DutyRecord>> {
        self.ensure_schema().await?;
        let now_micros = now.timestamp_micros();
        let rows = sqlx::query(
            "SELECT id, tenant_id, worker_id, cron, entry_action, description, \
             enabled, paused_until, last_fired_at, created_at \
             FROM role_duty WHERE enabled = 1 \
             AND (paused_until IS NULL OR paused_until <= ?) \
             ORDER BY created_at ASC, id ASC",
        )
        .bind(now_micros)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list active duties: {e}")))?;
        rows.into_iter().map(row_to_duty).collect()
    }

    /// Set `paused_until` on a duty. Pass `None` to clear (resume).
    pub async fn set_paused(&self, id: &str, until: Option<DateTime<Utc>>) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query("UPDATE role_duty SET paused_until = ? WHERE id = ?")
            .bind(until.map(|d| d.timestamp_micros()))
            .bind(id)
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("set paused_until: {e}")))?;
        Ok(())
    }

    /// Update `last_fired_at`. Called after a successful fire.
    pub async fn mark_fired(&self, id: &str, fired_at: DateTime<Utc>) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query("UPDATE role_duty SET last_fired_at = ? WHERE id = ?")
            .bind(fired_at.timestamp_micros())
            .bind(id)
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("mark fired: {e}")))?;
        Ok(())
    }

    /// Record the (duty -> session) attribution row.
    pub async fn link_fire(
        &self,
        duty_id: &str,
        session_id: &SessionId,
        fired_at: DateTime<Utc>,
        decision: FireDecision,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query(
            "INSERT OR IGNORE INTO duty_fired_session \
             (duty_id, session_id, fired_at, decision) VALUES (?, ?, ?, ?)",
        )
        .bind(duty_id)
        .bind(session_id.as_str())
        .bind(fired_at.timestamp_micros())
        .bind(decision.label())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("link fire: {e}")))?;
        Ok(())
    }

    /// All sessions ever spawned by one duty, newest first.
    pub async fn sessions_for_duty(
        &self,
        duty_id: &str,
    ) -> ArgoResult<Vec<(SessionId, DateTime<Utc>, String)>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT session_id, fired_at, decision FROM duty_fired_session \
             WHERE duty_id = ? ORDER BY fired_at DESC",
        )
        .bind(duty_id)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("sessions_for_duty: {e}")))?;
        rows.into_iter()
            .map(|row| {
                let sid: String = row.try_get("session_id").map_err(map_provider)?;
                let fired_micros: i64 = row.try_get("fired_at").map_err(map_provider)?;
                let decision: String = row.try_get("decision").map_err(map_provider)?;
                let fired_at = micros_to_utc(fired_micros, "fired_at")?;
                Ok((SessionId::from_string(sid), fired_at, decision))
            })
            .collect()
    }
}

pub(crate) fn row_to_duty(row: sqlx::sqlite::SqliteRow) -> ArgoResult<DutyRecord> {
    let tenant_blob: Vec<u8> = row.try_get("tenant_id").map_err(map_provider)?;
    let tenant_id = TenantId(
        Uuid::from_slice(&tenant_blob).map_err(|e| ArgoError::Schema(format!("tenant: {e}")))?,
    );
    let worker_str: String = row.try_get("worker_id").map_err(map_provider)?;
    let worker_id =
        WorkerId::from_str(&worker_str).map_err(|e| ArgoError::Schema(format!("worker: {e}")))?;
    let enabled_int: i64 = row.try_get("enabled").map_err(map_provider)?;
    let paused: Option<i64> = row.try_get("paused_until").map_err(map_provider)?;
    let last_fired: Option<i64> = row.try_get("last_fired_at").map_err(map_provider)?;
    let created: i64 = row.try_get("created_at").map_err(map_provider)?;
    Ok(DutyRecord {
        id: row.try_get("id").map_err(map_provider)?,
        tenant_id,
        worker_id,
        cron: row.try_get("cron").map_err(map_provider)?,
        entry_action: row.try_get("entry_action").map_err(map_provider)?,
        description: row.try_get("description").map_err(map_provider)?,
        enabled: enabled_int != 0,
        paused_until: paused
            .map(|v| micros_to_utc(v, "paused_until"))
            .transpose()?,
        last_fired_at: last_fired
            .map(|v| micros_to_utc(v, "last_fired_at"))
            .transpose()?,
        created_at: micros_to_utc(created, "created_at")?,
        gate: GateConfig::default(),
    })
}

pub(crate) fn map_provider(e: sqlx::Error) -> ArgoError {
    ArgoError::Provider(format!("duty row: {e}"))
}

pub(crate) fn micros_to_utc(v: i64, field: &'static str) -> ArgoResult<DateTime<Utc>> {
    Utc.timestamp_micros(v)
        .single()
        .ok_or_else(|| ArgoError::Schema(format!("{field} out of range")))
}
