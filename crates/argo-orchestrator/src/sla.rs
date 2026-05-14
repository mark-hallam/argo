//! SLA definitions, persistence, and breach detection.
//!
//! The module is intentionally transport-agnostic. Gateway handlers
//! parse HTTP and call [`SlaStore`]; scheduler/runtime code can call
//! [`SlaTracker`] directly when evaluating in-flight work.
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

const SLA_SCHEMA: &str = include_str!("../migrations/78__sla.sql");

/// What an SLA attaches to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlaTarget {
    /// A whole worker should respond within the configured latency.
    Worker(WorkerId),
    /// One duty should complete within the configured latency.
    Duty(String),
    /// One skill should run within the configured latency.
    Skill(String),
}

impl SlaTarget {
    /// Stable target kind used in storage and JSON.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Worker(_) => "worker",
            Self::Duty(_) => "duty",
            Self::Skill(_) => "skill",
        }
    }

    /// Stable target id used in storage and JSON.
    #[must_use]
    pub fn id(&self) -> String {
        match self {
            Self::Worker(id) => id.to_string(),
            Self::Duty(id) | Self::Skill(id) => id.clone(),
        }
    }

    /// Hydrate from stored kind/id fields.
    pub fn from_parts(kind: &str, id: &str) -> ArgoResult<Self> {
        match kind {
            "worker" => WorkerId::from_str(id)
                .map(Self::Worker)
                .map_err(|e| ArgoError::Schema(format!("worker target: {e}"))),
            "duty" if !id.trim().is_empty() => Ok(Self::Duty(id.to_string())),
            "skill" if !id.trim().is_empty() => Ok(Self::Skill(id.to_string())),
            _ => Err(ArgoError::Schema(format!("unknown SLA target '{kind}'"))),
        }
    }
}

/// Escalation policy applied when an SLA breaches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum EscalationPolicy {
    /// Notify a channel such as `slack:#ops`.
    Notify(String),
    /// Reroute to another worker, queue, or escalation alias.
    Reroute(String),
    /// Fail the run when the threshold is breached.
    Fail,
}

impl EscalationPolicy {
    /// Compact storage representation.
    #[must_use]
    pub fn encode(&self) -> String {
        match self {
            Self::Notify(v) => format!("notify:{v}"),
            Self::Reroute(v) => format!("reroute:{v}"),
            Self::Fail => "fail".to_string(),
        }
    }

    /// Decode from storage.
    #[must_use]
    pub fn decode(raw: &str) -> Self {
        raw.strip_prefix("notify:")
            .map(|v| Self::Notify(v.to_string()))
            .or_else(|| {
                raw.strip_prefix("reroute:")
                    .map(|v| Self::Reroute(v.to_string()))
            })
            .unwrap_or(Self::Fail)
    }
}

/// Concrete breach action emitted for operators and follow-up workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreachAction {
    /// Notify only; the run continues.
    Notify,
    /// Reroute to the policy target.
    Reroute,
    /// Fail the run.
    Fail,
}

impl BreachAction {
    /// Storage label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Notify => "notify",
            Self::Reroute => "reroute",
            Self::Fail => "fail",
        }
    }

    fn from_label(raw: &str) -> Self {
        match raw {
            "reroute" => Self::Reroute,
            "fail" => Self::Fail,
            _ => Self::Notify,
        }
    }
}

/// Persisted SLA definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sla {
    /// UUIDv7 string id.
    pub id: String,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Worker, duty, or skill target.
    pub target: SlaTarget,
    /// Maximum allowed latency before breach.
    pub target_latency_ms: u64,
    /// Escalation policy honored on breach.
    pub escalation_policy: EscalationPolicy,
    /// Concrete action to emit on breach.
    pub breach_action: BreachAction,
    /// Disabled SLAs are retained but ignored by trackers.
    pub enabled: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Sla {
    /// Create a fresh enabled SLA definition.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        target: SlaTarget,
        target_latency_ms: u64,
        escalation_policy: EscalationPolicy,
        breach_action: BreachAction,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7().to_string(),
            tenant_id,
            target,
            target_latency_ms,
            escalation_policy,
            breach_action,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Observed in-flight work used by the tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRun {
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Target being evaluated.
    pub target: SlaTarget,
    /// When the work started.
    pub started_at: DateTime<Utc>,
    /// Evaluation timestamp.
    pub observed_at: DateTime<Utc>,
}

/// Emitted SLA breach event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlaBreachEvent {
    /// UUIDv7 string id.
    pub id: String,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Definition that breached.
    pub sla_id: String,
    /// Target that breached.
    pub target: SlaTarget,
    /// Observed latency.
    pub observed_latency_ms: u64,
    /// Threshold copied from the SLA.
    pub threshold_ms: u64,
    /// Action selected from the SLA.
    pub action: BreachAction,
    /// Emission timestamp.
    pub emitted_at: DateTime<Utc>,
}

/// Pure evaluator for SLA breaches.
pub struct SlaTracker;

impl SlaTracker {
    /// Return a breach event if `run` exceeds `sla`.
    #[must_use]
    pub fn evaluate(sla: &Sla, run: &ObservedRun) -> Option<SlaBreachEvent> {
        if !sla.enabled || sla.tenant_id != run.tenant_id || sla.target != run.target {
            return None;
        }
        let latency_ms = run
            .observed_at
            .signed_duration_since(run.started_at)
            .num_milliseconds()
            .max(0);
        let latency = u64::try_from(latency_ms).unwrap_or(0);
        (latency > sla.target_latency_ms).then(|| SlaBreachEvent {
            id: Uuid::now_v7().to_string(),
            tenant_id: sla.tenant_id,
            sla_id: sla.id.clone(),
            target: sla.target.clone(),
            observed_latency_ms: latency,
            threshold_ms: sla.target_latency_ms,
            action: sla.breach_action,
            emitted_at: run.observed_at,
        })
    }
}

/// SQLite-backed SLA CRUD and breach-event store.
pub struct SlaStore {
    store: Arc<SessionStore>,
}

impl SlaStore {
    /// Build over an open [`SessionStore`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Idempotently create the SLA tables.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        let cleaned = SLA_SCHEMA
            .lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        for stmt in cleaned
            .split_terminator(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            sqlx::query(stmt)
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("sla schema: {e}")))?;
        }
        Ok(())
    }

    /// Create one SLA definition.
    pub async fn create(&self, sla: &Sla) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query(
            "INSERT INTO sla_definition (id, tenant_id, target_kind, target_id, target_latency_ms, \
             escalation_policy, breach_action, enabled, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&sla.id)
        .bind(sla.tenant_id.0.as_bytes().as_slice())
        .bind(sla.target.kind())
        .bind(sla.target.id())
        .bind(i64::try_from(sla.target_latency_ms).unwrap_or(i64::MAX))
        .bind(sla.escalation_policy.encode())
        .bind(sla.breach_action.label())
        .bind(i64::from(sla.enabled))
        .bind(sla.created_at.timestamp_micros())
        .bind(sla.updated_at.timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("create sla: {e}")))?;
        Ok(())
    }

    /// Fetch one SLA by tenant and id.
    pub async fn get(&self, tenant: TenantId, id: &str) -> ArgoResult<Option<Sla>> {
        self.ensure_schema().await?;
        let row = sqlx::query("SELECT * FROM sla_definition WHERE tenant_id = ? AND id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .bind(id)
            .fetch_optional(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("get sla: {e}")))?;
        row.map(row_to_sla).transpose()
    }

    /// List enabled and disabled SLAs for a tenant.
    pub async fn list(&self, tenant: TenantId) -> ArgoResult<Vec<Sla>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT * FROM sla_definition WHERE tenant_id = ? ORDER BY created_at DESC, id ASC",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list sla: {e}")))?;
        rows.into_iter().map(row_to_sla).collect()
    }

    /// Replace mutable fields on an SLA.
    pub async fn update(&self, sla: &Sla) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let affected = sqlx::query(
            "UPDATE sla_definition SET target_latency_ms = ?, escalation_policy = ?, \
             breach_action = ?, enabled = ?, updated_at = ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(i64::try_from(sla.target_latency_ms).unwrap_or(i64::MAX))
        .bind(sla.escalation_policy.encode())
        .bind(sla.breach_action.label())
        .bind(i64::from(sla.enabled))
        .bind(sla.updated_at.timestamp_micros())
        .bind(sla.tenant_id.0.as_bytes().as_slice())
        .bind(&sla.id)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("update sla: {e}")))?
        .rows_affected();
        if affected == 0 {
            return Err(ArgoError::Invariant(format!("sla '{}' not found", sla.id)));
        }
        Ok(())
    }

    /// Delete one SLA definition.
    pub async fn delete(&self, tenant: TenantId, id: &str) -> ArgoResult<bool> {
        self.ensure_schema().await?;
        let n = sqlx::query("DELETE FROM sla_definition WHERE tenant_id = ? AND id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .bind(id)
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("delete sla: {e}")))?
            .rows_affected();
        Ok(n > 0)
    }

    /// Persist a breach event.
    pub async fn record_breach(&self, event: &SlaBreachEvent) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query(
            "INSERT INTO sla_breach_event (id, tenant_id, sla_id, target_kind, target_id, \
             observed_latency_ms, threshold_ms, action, emitted_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(event.tenant_id.0.as_bytes().as_slice())
        .bind(&event.sla_id)
        .bind(event.target.kind())
        .bind(event.target.id())
        .bind(i64::try_from(event.observed_latency_ms).unwrap_or(i64::MAX))
        .bind(i64::try_from(event.threshold_ms).unwrap_or(i64::MAX))
        .bind(event.action.label())
        .bind(event.emitted_at.timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("record sla breach: {e}")))?;
        Ok(())
    }
}

fn row_to_sla(row: sqlx::sqlite::SqliteRow) -> ArgoResult<Sla> {
    let tenant_blob: Vec<u8> = row.try_get("tenant_id").map_err(db)?;
    let tenant_id = TenantId(
        Uuid::from_slice(&tenant_blob).map_err(|e| ArgoError::Schema(format!("tenant: {e}")))?,
    );
    let kind: String = row.try_get("target_kind").map_err(db)?;
    let target_id: String = row.try_get("target_id").map_err(db)?;
    let enabled: i64 = row.try_get("enabled").map_err(db)?;
    let latency: i64 = row.try_get("target_latency_ms").map_err(db)?;
    Ok(Sla {
        id: row.try_get("id").map_err(db)?,
        tenant_id,
        target: SlaTarget::from_parts(&kind, &target_id)?,
        target_latency_ms: u64::try_from(latency).unwrap_or(0),
        escalation_policy: EscalationPolicy::decode(
            &row.try_get::<String, _>("escalation_policy").map_err(db)?,
        ),
        breach_action: BreachAction::from_label(
            &row.try_get::<String, _>("breach_action").map_err(db)?,
        ),
        enabled: enabled != 0,
        created_at: micros(row.try_get("created_at").map_err(db)?)?,
        updated_at: micros(row.try_get("updated_at").map_err(db)?)?,
    })
}

fn micros(v: i64) -> ArgoResult<DateTime<Utc>> {
    DateTime::from_timestamp_micros(v)
        .ok_or_else(|| ArgoError::Provider(format!("timestamp out of range: {v}")))
}

fn db(e: impl std::fmt::Display) -> ArgoError {
    ArgoError::Provider(e.to_string())
}
