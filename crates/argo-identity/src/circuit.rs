//! Worker circuit breaker persistence.
//!
//! The circuit table is an append-only audit trail. The latest
//! row per worker is the current state; closed rows can still
//! carry the consecutive-failure counter so the next failed
//! transition can trip without mutating prior history.

use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

const CIRCUIT_SCHEMA: &str =
    include_str!("../../argo-store/migrations/17__worker_circuit_history.sql");
const DEFAULT_THRESHOLD: u32 = 3;

/// Runtime dispatch circuit for a worker.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CircuitState {
    /// Normal dispatch path.
    #[default]
    Closed,
    /// Dispatch is paused until an operator resumes the worker.
    Open {
        /// UTC time the circuit tripped.
        tripped_at: DateTime<Utc>,
        /// Human-readable failure summary.
        reason: String,
    },
    /// One operator-resumed probe action is allowed.
    HalfOpen,
}

impl CircuitState {
    /// Stable label persisted into `worker_circuit_history`.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open { .. } => "open",
            Self::HalfOpen => "half_open",
        }
    }
}

/// Circuit threshold from `ARGO_CIRCUIT_THRESHOLD`, defaulting
/// to 3. Invalid or zero values fall back to the default.
#[must_use]
pub fn threshold_from_env() -> u32 {
    threshold_from_str(std::env::var("ARGO_CIRCUIT_THRESHOLD").ok().as_deref())
}

#[must_use]
fn threshold_from_str(raw: Option<&str>) -> u32 {
    raw.and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_THRESHOLD)
}

/// Ensure the audit table exists.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    for stmt in CIRCUIT_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql)
                .execute(pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("worker circuit schema: {e}")))?;
        }
    }
    Ok(())
}

/// Return the latest state for a worker. Workers with no circuit
/// rows are closed.
pub async fn state_for(pool: &SqlitePool, worker_id: &str) -> ArgoResult<CircuitState> {
    ensure_schema(pool).await?;
    Ok(latest_row(pool, worker_id)
        .await?
        .map_or(CircuitState::Closed, |row| row.state))
}

/// Persist a failed action transition and trip when the
/// consecutive-failure threshold is reached.
pub async fn record_failure(
    pool: &SqlitePool,
    worker_id: &str,
    reason: &str,
) -> ArgoResult<CircuitState> {
    record_failure_with_threshold(pool, worker_id, reason, threshold_from_env(), Utc::now()).await
}

async fn record_failure_with_threshold(
    pool: &SqlitePool,
    worker_id: &str,
    reason: &str,
    threshold: u32,
    now: DateTime<Utc>,
) -> ArgoResult<CircuitState> {
    ensure_schema(pool).await?;
    let threshold = threshold.max(1);
    let latest = latest_row(pool, worker_id).await?;
    if let Some(row) = latest.as_ref() {
        if matches!(row.state, CircuitState::Open { .. }) {
            return Ok(row.state.clone());
        }
    }
    let next_count = match latest {
        Some(row) if row.state == CircuitState::HalfOpen => threshold,
        Some(row) => row.failure_count.saturating_add(1),
        None => 1,
    };
    let state = if next_count >= threshold {
        CircuitState::Open {
            tripped_at: now,
            reason: reason.to_string(),
        }
    } else {
        CircuitState::Closed
    };
    insert_state(pool, worker_id, &state, next_count, Some(reason), now).await?;
    Ok(state)
}

/// Persist a successful action transition. Any prior consecutive
/// failures are reset; a half-open probe success closes the circuit.
pub async fn record_success(pool: &SqlitePool, worker_id: &str) -> ArgoResult<CircuitState> {
    ensure_schema(pool).await?;
    let latest = latest_row(pool, worker_id).await?;
    if latest
        .as_ref()
        .is_some_and(|row| row.failure_count > 0 || row.state == CircuitState::HalfOpen)
    {
        insert_state(pool, worker_id, &CircuitState::Closed, 0, None, Utc::now()).await?;
    }
    Ok(CircuitState::Closed)
}

/// Operator resume: an open circuit becomes half-open. Closed
/// workers refuse resume so accidental commands are visible.
pub async fn resume(
    pool: &SqlitePool,
    worker_id: &str,
    reason: Option<&str>,
) -> ArgoResult<CircuitState> {
    ensure_schema(pool).await?;
    match state_for(pool, worker_id).await? {
        CircuitState::Open { .. } => {
            insert_state(
                pool,
                worker_id,
                &CircuitState::HalfOpen,
                0,
                reason,
                Utc::now(),
            )
            .await?;
            Ok(CircuitState::HalfOpen)
        }
        other => Err(ArgoError::Provider(format!(
            "worker {worker_id} circuit is {}, not open",
            other.as_str()
        ))),
    }
}

#[derive(Clone)]
struct CircuitRow {
    state: CircuitState,
    failure_count: u32,
}

async fn latest_row(pool: &SqlitePool, worker_id: &str) -> ArgoResult<Option<CircuitRow>> {
    let row = sqlx::query(
        "SELECT state, failure_count, reason, created_at
         FROM worker_circuit_history
         WHERE worker_id = ?
         ORDER BY created_at DESC, id DESC
         LIMIT 1",
    )
    .bind(worker_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker circuit read: {e}")))?;
    row.map(row_to_circuit).transpose()
}

fn row_to_circuit(row: sqlx::sqlite::SqliteRow) -> ArgoResult<CircuitRow> {
    let label: String = row.get("state");
    let failure_count: i64 = row.get("failure_count");
    let reason: Option<String> = row.get("reason");
    let created_at: i64 = row.get("created_at");
    let state = match label.as_str() {
        "closed" => CircuitState::Closed,
        "half_open" => CircuitState::HalfOpen,
        "open" => CircuitState::Open {
            tripped_at: Utc
                .timestamp_opt(created_at, 0)
                .single()
                .unwrap_or_else(Utc::now),
            reason: reason.unwrap_or_else(|| "consecutive failures".to_string()),
        },
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown worker circuit state '{other}'"
            )))
        }
    };
    Ok(CircuitRow {
        state,
        failure_count: u32::try_from(failure_count).unwrap_or(0),
    })
}

async fn insert_state(
    pool: &SqlitePool,
    worker_id: &str,
    state: &CircuitState,
    failure_count: u32,
    reason: Option<&str>,
    now: DateTime<Utc>,
) -> ArgoResult<()> {
    let reason = match state {
        CircuitState::Open { reason, .. } => Some(reason.as_str()),
        _ => reason,
    };
    sqlx::query(
        "INSERT INTO worker_circuit_history
         (worker_id, state, failure_count, reason, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(worker_id)
    .bind(state.as_str())
    .bind(i64::from(failure_count))
    .bind(reason)
    .bind(now.timestamp())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker circuit write: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionStore;
    use chrono::TimeZone;

    async fn pool() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        ensure_schema(store.pool()).await.unwrap();
        store
    }

    fn at(sec: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(sec, 0).single().unwrap()
    }

    #[test]
    fn threshold_defaults_and_rejects_invalid_values() {
        assert_eq!(threshold_from_str(None), 3);
        assert_eq!(threshold_from_str(Some("0")), 3);
        assert_eq!(threshold_from_str(Some("nope")), 3);
        assert_eq!(threshold_from_str(Some("5")), 5);
    }

    #[tokio::test]
    async fn missing_worker_state_is_closed() {
        let store = pool().await;
        assert_eq!(
            state_for(store.pool(), "worker-a").await.unwrap(),
            CircuitState::Closed
        );
    }

    #[tokio::test]
    async fn third_consecutive_failure_opens_circuit() {
        let store = pool().await;
        let p = store.pool();
        assert_eq!(
            record_failure_with_threshold(p, "worker-a", "one", 3, at(1))
                .await
                .unwrap(),
            CircuitState::Closed
        );
        record_failure_with_threshold(p, "worker-a", "two", 3, at(2))
            .await
            .unwrap();
        let state = record_failure_with_threshold(p, "worker-a", "three", 3, at(3))
            .await
            .unwrap();
        assert!(matches!(state, CircuitState::Open { reason, .. } if reason == "three"));
    }

    #[tokio::test]
    async fn resume_moves_open_to_half_open() {
        let store = pool().await;
        let p = store.pool();
        record_failure_with_threshold(p, "worker-a", "boom", 1, at(1))
            .await
            .unwrap();
        assert_eq!(
            resume(p, "worker-a", Some("operator")).await.unwrap(),
            CircuitState::HalfOpen
        );
    }

    #[tokio::test]
    async fn half_open_success_closes_and_resets_counter() {
        let store = pool().await;
        let p = store.pool();
        record_failure_with_threshold(p, "worker-a", "boom", 1, at(1))
            .await
            .unwrap();
        resume(p, "worker-a", None).await.unwrap();
        assert_eq!(
            record_success(p, "worker-a").await.unwrap(),
            CircuitState::Closed
        );
        record_failure_with_threshold(p, "worker-a", "one", 2, at(2))
            .await
            .unwrap();
        assert_eq!(
            state_for(p, "worker-a").await.unwrap(),
            CircuitState::Closed
        );
    }

    #[tokio::test]
    async fn half_open_failure_reopens_immediately() {
        let store = pool().await;
        let p = store.pool();
        record_failure_with_threshold(p, "worker-a", "boom", 2, at(1))
            .await
            .unwrap();
        record_failure_with_threshold(p, "worker-a", "again", 2, at(2))
            .await
            .unwrap();
        resume(p, "worker-a", None).await.unwrap();
        let state = record_failure_with_threshold(p, "worker-a", "probe failed", 3, at(3))
            .await
            .unwrap();
        assert!(matches!(state, CircuitState::Open { reason, .. } if reason == "probe failed"));
    }
}
