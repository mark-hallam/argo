//! Per-skill retry policies for orchestrated skill runs.
//!
//! The policy is intentionally skill-scoped: orchestration decides if
//! a failed skill run gets another attempt, while the low-level delay
//! calculation reuses `argo-mcp-connector-shared`'s exponential backoff
//! primitive.
use argo_mcp_connector_shared::retry as shared_retry;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const RETRY_SCHEMA: &str = include_str!("../migrations/79__retry_policy.sql");

/// Delay strategy applied between skill attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryBackoff {
    /// Retry immediately with no sleep.
    None,
    /// Exponential backoff using the shared MCP connector primitive.
    Exponential,
}

impl RetryBackoff {
    /// Storage label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Exponential => "exponential",
        }
    }

    fn from_label(raw: &str) -> Self {
        match raw {
            "exponential" => Self::Exponential,
            _ => Self::None,
        }
    }
}

/// Failure class that can be matched by a retry policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryFailureClass {
    /// Transient transport or upstream failure.
    Transient,
    /// Rate limit or quota pressure.
    RateLimited,
    /// Deadline or timeout.
    Timeout,
    /// Internal worker or skill execution failure.
    Internal,
}

/// Skill failure surfaced to the retry runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryFailure {
    /// Machine-readable failure class.
    pub class: RetryFailureClass,
    /// Human-readable diagnostic for audit trails.
    pub message: String,
}

impl RetryFailure {
    /// Build a classified retry failure.
    #[must_use]
    pub fn new(class: RetryFailureClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }
}

/// Persisted retry policy for one skill IR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRetryPolicy {
    /// UUIDv7 string id.
    pub id: String,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Skill identifier.
    pub skill_id: String,
    /// Optional skill-IR version this policy applies to.
    pub ir_version: Option<String>,
    /// Total attempts including the first try.
    pub max_attempts: u32,
    /// Delay strategy between retries.
    pub backoff: RetryBackoff,
    /// Delay before the first retry.
    pub initial_delay_ms: u64,
    /// Maximum per-retry delay.
    pub max_delay_ms: u64,
    /// Failure classes that should be retried.
    pub retry_on: Vec<RetryFailureClass>,
    /// Disabled policies are retained but ignored by runners.
    pub enabled: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl SkillRetryPolicy {
    /// Create a default enabled policy for a skill.
    #[must_use]
    pub fn new(tenant_id: TenantId, skill_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7().to_string(),
            tenant_id,
            skill_id: skill_id.into(),
            ir_version: None,
            max_attempts: 3,
            backoff: RetryBackoff::Exponential,
            initial_delay_ms: 250,
            max_delay_ms: 5_000,
            retry_on: vec![
                RetryFailureClass::Transient,
                RetryFailureClass::RateLimited,
                RetryFailureClass::Timeout,
            ],
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Convert to the shared MCP retry policy shape.
    #[must_use]
    pub fn shared_policy(&self) -> shared_retry::RetryPolicy {
        let (initial, max) = match self.backoff {
            RetryBackoff::None => (0, 0),
            RetryBackoff::Exponential => (self.initial_delay_ms, self.max_delay_ms),
        };
        shared_retry::RetryPolicy {
            max_attempts: self.max_attempts.max(1),
            initial_delay_ms: initial,
            multiplier: 2.0,
            max_delay_ms: max,
            jitter_ms: 0,
        }
    }

    /// Deterministic delay for a retry attempt using shared backoff math.
    #[must_use]
    pub fn delay_ms_for_attempt(&self, attempt: u32) -> u64 {
        shared_retry::backoff_delay_ms(&self.shared_policy(), attempt, 0)
    }

    /// Return true if this policy should retry `failure`.
    #[must_use]
    pub fn retries(&self, failure: &RetryFailure) -> bool {
        self.enabled && self.retry_on.contains(&failure.class)
    }
}

/// Run a skill operation under a retry policy.
pub async fn run_skill_with_retry<F, Fut, T>(
    policy: &SkillRetryPolicy,
    mut op: F,
) -> Result<T, RetryFailure>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, RetryFailure>>,
{
    let max_attempts = policy.max_attempts.max(1);
    let mut attempts = 0;
    loop {
        attempts += 1;
        match op().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempts >= max_attempts || !policy.retries(&err) {
                    return Err(err);
                }
                let delay_ms = policy.delay_ms_for_attempt(attempts);
                if delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
}

/// SQLite-backed retry policy store.
pub struct RetryPolicyStore {
    store: Arc<SessionStore>,
}

impl RetryPolicyStore {
    /// Build over an open [`SessionStore`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Idempotently create the retry policy table.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        let cleaned = RETRY_SCHEMA
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
                .map_err(|e| ArgoError::Provider(format!("retry schema: {e}")))?;
        }
        Ok(())
    }

    /// Insert or replace one skill retry policy.
    pub async fn upsert(&self, policy: &SkillRetryPolicy) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let retry_on = serde_json::to_string(&policy.retry_on)
            .map_err(|e| ArgoError::Schema(format!("retry_on: {e}")))?;
        sqlx::query(
            "INSERT OR REPLACE INTO skill_retry_policy \
             (id, tenant_id, skill_id, ir_version, max_attempts, backoff, initial_delay_ms, \
              max_delay_ms, retry_on, enabled, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&policy.id)
        .bind(policy.tenant_id.0.as_bytes().as_slice())
        .bind(&policy.skill_id)
        .bind(&policy.ir_version)
        .bind(i64::from(policy.max_attempts))
        .bind(policy.backoff.label())
        .bind(i64::try_from(policy.initial_delay_ms).unwrap_or(i64::MAX))
        .bind(i64::try_from(policy.max_delay_ms).unwrap_or(i64::MAX))
        .bind(retry_on)
        .bind(i64::from(policy.enabled))
        .bind(policy.created_at.timestamp_micros())
        .bind(policy.updated_at.timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("upsert retry policy: {e}")))?;
        Ok(())
    }

    /// Fetch one policy by tenant and id.
    pub async fn get(&self, tenant: TenantId, id: &str) -> ArgoResult<Option<SkillRetryPolicy>> {
        self.ensure_schema().await?;
        let row = sqlx::query("SELECT * FROM skill_retry_policy WHERE tenant_id = ? AND id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .bind(id)
            .fetch_optional(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("get retry policy: {e}")))?;
        row.map(row_to_policy).transpose()
    }

    /// List policies for a tenant and skill.
    pub async fn list_for_skill(
        &self,
        tenant: TenantId,
        skill_id: &str,
    ) -> ArgoResult<Vec<SkillRetryPolicy>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT * FROM skill_retry_policy WHERE tenant_id = ? AND skill_id = ? \
             ORDER BY created_at DESC, id ASC",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .bind(skill_id)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list retry policies: {e}")))?;
        rows.into_iter().map(row_to_policy).collect()
    }
}

fn row_to_policy(row: sqlx::sqlite::SqliteRow) -> ArgoResult<SkillRetryPolicy> {
    let tenant_blob: Vec<u8> = row.try_get("tenant_id").map_err(db)?;
    let tenant_id = TenantId(
        Uuid::from_slice(&tenant_blob).map_err(|e| ArgoError::Schema(format!("tenant: {e}")))?,
    );
    let retry_on_raw: String = row.try_get("retry_on").map_err(db)?;
    let retry_on = serde_json::from_str(&retry_on_raw)
        .map_err(|e| ArgoError::Schema(format!("retry_on: {e}")))?;
    let max_attempts: i64 = row.try_get("max_attempts").map_err(db)?;
    let initial_delay_ms: i64 = row.try_get("initial_delay_ms").map_err(db)?;
    let max_delay_ms: i64 = row.try_get("max_delay_ms").map_err(db)?;
    let enabled: i64 = row.try_get("enabled").map_err(db)?;
    Ok(SkillRetryPolicy {
        id: row.try_get("id").map_err(db)?,
        tenant_id,
        skill_id: row.try_get("skill_id").map_err(db)?,
        ir_version: row.try_get("ir_version").map_err(db)?,
        max_attempts: u32::try_from(max_attempts).unwrap_or(1),
        backoff: RetryBackoff::from_label(&row.try_get::<String, _>("backoff").map_err(db)?),
        initial_delay_ms: u64::try_from(initial_delay_ms).unwrap_or(0),
        max_delay_ms: u64::try_from(max_delay_ms).unwrap_or(0),
        retry_on,
        enabled: enabled != 0,
        created_at: micros(row.try_get("created_at").map_err(db)?)?,
        updated_at: micros(row.try_get("updated_at").map_err(db)?)?,
    })
}

fn micros(v: i64) -> ArgoResult<DateTime<Utc>> {
    DateTime::from_timestamp_micros(v)
        .ok_or_else(|| ArgoError::Invariant(format!("timestamp out of range: {v}")))
}

fn db(e: impl std::fmt::Display) -> ArgoError {
    ArgoError::Provider(e.to_string())
}
