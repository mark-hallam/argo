//! Periodic SIEM dispatcher — tails `audit_record` by `seq`, batches
//! per sink, ships with retry-on-5xx + exponential backoff, persists
//! the high-water `seq` to `siem_dispatch_cursor`.
//!
//! Crash + restart resumes from `last_seq + 1`; the cursor advances
//! only after a successful `ship`, so a mid-flight crash re-ships at
//! most the last in-flight batch (at-least-once; sinks that dedupe
//! on `id` collapse the re-ship). The cursor table is created via
//! `CREATE TABLE IF NOT EXISTS` at startup so the dispatcher works
//! on any DB with the base `audit_record` migration, without a
//! coupled `argo-store` migration register entry.
use crate::sink::{AuditEvent, SiemSink, SinkError};
use argo_store::SessionStore;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Default poll interval when the config does not override.
pub const DEFAULT_INTERVAL_SECS: u64 = 15;
/// Initial backoff for the retry-on-5xx loop.
const INITIAL_BACKOFF: Duration = Duration::from_millis(500);
/// Backoff ceiling — even pathological 5xx loops won't pause the
/// next tick longer than this.
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// Maximum retries per batch before the dispatcher gives up,
/// advances the cursor past the failed batch, and writes a
/// `siem.dispatch_failed` audit row. v1 ships 5 — enough to ride
/// out a brief outage, short enough that a permanent failure
/// doesn't stall the cursor forever.
const MAX_RETRIES: u32 = 5;

/// Configuration for [`spawn_siem_dispatcher`].
#[derive(Debug, Clone)]
pub struct SiemDispatcherConfig {
    /// Poll interval between cursor sweeps.
    pub interval: Duration,
}

impl Default for SiemDispatcherConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(DEFAULT_INTERVAL_SECS),
        }
    }
}

/// Dispatcher state. Public so tests can drive `sweep_once` directly
/// without spawning a task.
pub struct SiemDispatcher {
    store: Arc<SessionStore>,
    sink: Arc<dyn SiemSink>,
}

impl SiemDispatcher {
    /// Build a dispatcher. `_config` is accepted to mirror
    /// [`spawn_siem_dispatcher`]; the interval is consumed by the
    /// spawn helper, not by `sweep_once`.
    #[must_use]
    pub fn new(
        store: Arc<SessionStore>,
        sink: Arc<dyn SiemSink>,
        _config: SiemDispatcherConfig,
    ) -> Self {
        Self { store, sink }
    }

    /// `CREATE TABLE IF NOT EXISTS` for the cursor row. Idempotent.
    ///
    /// # Errors
    /// Returns the sqlx error if the DDL fails.
    pub async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS siem_dispatch_cursor (\n                backend         TEXT NOT NULL,\n                endpoint        TEXT NOT NULL,\n                last_seq        INTEGER NOT NULL,\n                last_shipped_at INTEGER NOT NULL,\n                PRIMARY KEY (backend, endpoint)\n            )",
        )
        .execute(self.store.pool())
        .await
        .map(|_| ())
    }

    /// Read the cursor for the configured sink. Returns 0 on first
    /// run (no row yet).
    ///
    /// # Errors
    /// Returns the sqlx error on read failure.
    pub async fn load_cursor(&self) -> Result<i64, sqlx::Error> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT last_seq FROM siem_dispatch_cursor WHERE backend = ? AND endpoint = ?",
        )
        .bind(self.sink.backend())
        .bind(self.sink.endpoint())
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row.map(|r| r.0).unwrap_or(0))
    }

    async fn save_cursor(&self, last_seq: i64) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO siem_dispatch_cursor (backend, endpoint, last_seq, last_shipped_at) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT(backend, endpoint) DO UPDATE SET \
                 last_seq = excluded.last_seq, \
                 last_shipped_at = excluded.last_shipped_at",
        )
        .bind(self.sink.backend())
        .bind(self.sink.endpoint())
        .bind(last_seq)
        .bind(now)
        .execute(self.store.pool())
        .await
        .map(|_| ())
    }

    async fn load_batch(&self, after_seq: i64, limit: usize) -> Result<Vec<AuditEvent>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT seq, id, session_id, kind, payload_json, created_at \
             FROM audit_record \
             WHERE seq > ? \
             ORDER BY seq ASC \
             LIMIT ?",
        )
        .bind(after_seq)
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(self.store.pool())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let seq: i64 = r.try_get("seq")?;
            let id_bytes: Vec<u8> = r.try_get("id")?;
            let session_bytes: Option<Vec<u8>> = r.try_get("session_id").ok();
            let kind: String = r.try_get("kind")?;
            let payload_json: String = r.try_get("payload_json")?;
            let created_at: i64 = r.try_get("created_at")?;
            let payload = serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null);
            out.push(AuditEvent {
                seq,
                id: hex::encode(&id_bytes),
                session_id: session_bytes.map(hex::encode),
                kind,
                payload,
                created_at,
            });
        }
        Ok(out)
    }

    /// Run one sweep: load cursor, fetch one batch, ship, advance.
    /// Returns `Some(last_seq)` on ship-or-permanent-fail, `None` if
    /// nothing was pending.
    ///
    /// # Errors
    /// Returns [`SinkError`]; store errors are wrapped as
    /// [`SinkError::Config`] to keep the surface uniform.
    pub async fn sweep_once(&self) -> Result<Option<i64>, SinkError> {
        let cursor = self
            .load_cursor()
            .await
            .map_err(|e| SinkError::Config(format!("load_cursor: {e}")))?;
        let limit = self.sink.max_batch();
        let batch = self
            .load_batch(cursor, limit)
            .await
            .map_err(|e| SinkError::Config(format!("load_batch: {e}")))?;
        if batch.is_empty() {
            return Ok(None);
        }
        let high_water = batch.last().map(|e| e.seq).unwrap_or(cursor);
        match self.ship_with_retries(batch).await {
            Ok(()) => {
                self.save_cursor(high_water)
                    .await
                    .map_err(|e| SinkError::Config(format!("save_cursor: {e}")))?;
                Ok(Some(high_water))
            }
            Err(SinkError::Permanent(msg)) => {
                tracing::error!(
                    target: "argo_siem::dispatcher",
                    backend = self.sink.backend(),
                    endpoint = self.sink.endpoint(),
                    last_seq = high_water,
                    "siem dispatch permanent failure; advancing cursor past poison batch: {msg}",
                );
                self.save_cursor(high_water)
                    .await
                    .map_err(|e| SinkError::Config(format!("save_cursor: {e}")))?;
                Ok(Some(high_water))
            }
            Err(e) => Err(e),
        }
    }

    async fn ship_with_retries(&self, events: Vec<AuditEvent>) -> Result<(), SinkError> {
        let mut delay = INITIAL_BACKOFF;
        let mut attempt: u32 = 0;
        loop {
            match self.sink.ship(events.clone()).await {
                Ok(()) => return Ok(()),
                Err(SinkError::Permanent(m)) => return Err(SinkError::Permanent(m)),
                Err(SinkError::Config(m)) => return Err(SinkError::Config(m)),
                Err(SinkError::Transient(m)) => {
                    attempt += 1;
                    if attempt >= MAX_RETRIES {
                        return Err(SinkError::Permanent(format!(
                            "transient retries exhausted ({MAX_RETRIES}): {m}"
                        )));
                    }
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(MAX_BACKOFF);
                }
            }
        }
    }
}

/// Spawn the dispatcher under a tokio task — mirrors
/// `spawn_handoff_sweeper` for gateway-side consistency.
#[must_use]
pub fn spawn_siem_dispatcher(
    store: Arc<SessionStore>,
    sink: Arc<dyn SiemSink>,
    config: SiemDispatcherConfig,
) -> JoinHandle<()> {
    let interval = config.interval;
    let dispatcher = SiemDispatcher::new(store, sink, config);
    tokio::spawn(async move {
        if let Err(e) = dispatcher.ensure_schema().await {
            tracing::error!(
                target: "argo_siem::dispatcher",
                error = %e,
                "siem dispatcher schema ensure failed; task exiting"
            );
            return;
        }
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match dispatcher.sweep_once().await {
                Ok(Some(seq)) => tracing::debug!(
                    target: "argo_siem::dispatcher",
                    last_seq = seq,
                    "siem batch shipped"
                ),
                Ok(None) => {}
                Err(e) => tracing::warn!(
                    target: "argo_siem::dispatcher",
                    error = %e,
                    "siem sweep failed; will retry on next tick"
                ),
            }
        }
    })
}

#[cfg(test)]
#[path = "dispatcher_tests.rs"]
mod tests;
