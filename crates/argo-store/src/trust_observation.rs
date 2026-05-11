//! Cross-tenant trust-graph observation persistence (Stream
//! `mission-cross-tenant-trust-graph`, MISSION_BACKLOG Section 3
//! Claim 4 #81).
//!
//! Append-only observation log. Each row is one quality observation
//! made by a consumer tenant against a skill shipped by a source
//! tenant. The in-memory graph in `argo_skill::trust_graph` consumes
//! a Vec of these rows and materializes pairwise weights via
//! `mean(quality_score)` clamped to `0.0..=1.0` with a linear
//! time-decay over `observed_at`.
//!
//! Schema is owned by `migrations/v50.rs` (raw SQL
//! `54__trust_observation.sql`); callers must run
//! `SessionStore::migrate` before calling [`record_observation`] or
//! [`list_observations_in_window`].
//!
//! # Example
//!
//! ```no_run
//! # use argo_store::trust_observation::{
//! #     list_observations_in_window, record_observation, NewTrustObservation,
//! # };
//! # use argo_types::{SkillId, TenantId};
//! # async fn run(store: &argo_store::SessionStore) -> argo_types::ArgoResult<()> {
//! let source = TenantId::new();
//! let consumer = TenantId::new();
//! let skill = SkillId::new();
//! let now = chrono::Utc::now();
//! record_observation(
//!     store,
//!     &NewTrustObservation {
//!         source_tenant: source,
//!         consumer_tenant: consumer,
//!         skill_id: skill,
//!         quality_score: 0.82,
//!         observed_at: now,
//!     },
//! )
//! .await?;
//! let _rows = list_observations_in_window(store, consumer, now - chrono::Duration::days(90)).await?;
//! # Ok(()) }
//! ```

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId, TenantId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// One persisted observation row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustObservationRow {
    /// UUIDv7 id (BLOB on disk).
    pub id: Uuid,
    /// Tenant that shipped the skill being observed.
    pub source_tenant: TenantId,
    /// Tenant that adopted the skill and reported a quality score.
    pub consumer_tenant: TenantId,
    /// Skill the observation references.
    pub skill_id: SkillId,
    /// Observed quality on the consumer side. Stored verbatim;
    /// callers clamp at the graph layer.
    pub quality_score: f32,
    /// Wall-clock time of the observation.
    pub observed_at: DateTime<Utc>,
}

/// Insert payload for [`record_observation`]. The `id` and storage
/// timestamp are auto-assigned (UUIDv7 + `observed_at` micros).
#[derive(Debug, Clone)]
pub struct NewTrustObservation {
    /// Tenant that shipped the skill being observed.
    pub source_tenant: TenantId,
    /// Tenant that adopted the skill and reported a quality score.
    pub consumer_tenant: TenantId,
    /// Skill the observation references.
    pub skill_id: SkillId,
    /// Observed quality on the consumer side.
    pub quality_score: f32,
    /// Wall-clock time of the observation.
    pub observed_at: DateTime<Utc>,
}

/// Append one row to `trust_observation`. Returns the assigned UUID
/// so callers can correlate the row with downstream audit records.
pub async fn record_observation(
    store: &SessionStore,
    new: &NewTrustObservation,
) -> ArgoResult<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO trust_observation \
         (id, source_tenant_id, consumer_tenant_id, skill_id, quality_score, observed_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(new.source_tenant.0.as_bytes().as_slice())
    .bind(new.consumer_tenant.0.as_bytes().as_slice())
    .bind(new.skill_id.0.as_bytes().as_slice())
    .bind(f64::from(new.quality_score))
    .bind(new.observed_at.timestamp_micros())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_observation insert: {e}")))?;
    Ok(id)
}

/// Return every row whose `observed_at >= since` and whose
/// `consumer_tenant_id == consumer`. Ordered oldest-first so the
/// graph builder can apply linear time-decay in a single forward
/// pass.
///
/// The brief's default query window is 90 days — callers pass
/// `Utc::now() - Duration::days(90)` for that behavior. The window
/// floor is a parameter rather than a constant so the CLI surface
/// can experiment with shorter / longer windows without forcing a
/// store change.
pub async fn list_observations_in_window(
    store: &SessionStore,
    consumer: TenantId,
    since: DateTime<Utc>,
) -> ArgoResult<Vec<TrustObservationRow>> {
    let rows = sqlx::query(
        "SELECT id, source_tenant_id, consumer_tenant_id, skill_id, \
                quality_score, observed_at \
         FROM trust_observation \
         WHERE consumer_tenant_id = ? AND observed_at >= ? \
         ORDER BY observed_at ASC",
    )
    .bind(consumer.0.as_bytes().as_slice())
    .bind(since.timestamp_micros())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_observation select: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(decode_row(&row)?);
    }
    Ok(out)
}

/// Return every row across all consumers since the window floor.
/// Used by `argo trust graph`/`argo trust composed` so a CLI run can
/// materialize the complete graph in one query, then call
/// `TrustGraph::from_observations` on the result.
pub async fn list_all_observations_since(
    store: &SessionStore,
    since: DateTime<Utc>,
) -> ArgoResult<Vec<TrustObservationRow>> {
    let rows = sqlx::query(
        "SELECT id, source_tenant_id, consumer_tenant_id, skill_id, \
                quality_score, observed_at \
         FROM trust_observation \
         WHERE observed_at >= ? \
         ORDER BY observed_at ASC",
    )
    .bind(since.timestamp_micros())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_observation select_all: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(decode_row(&row)?);
    }
    Ok(out)
}

fn decode_row(row: &sqlx::sqlite::SqliteRow) -> ArgoResult<TrustObservationRow> {
    let id_bytes: Vec<u8> = row.get("id");
    let source_bytes: Vec<u8> = row.get("source_tenant_id");
    let consumer_bytes: Vec<u8> = row.get("consumer_tenant_id");
    let skill_bytes: Vec<u8> = row.get("skill_id");
    let quality: f64 = row.get("quality_score");
    let observed_at_micros: i64 = row.get("observed_at");

    let id = Uuid::from_slice(&id_bytes)
        .map_err(|e| ArgoError::Provider(format!("trust_observation id from db: {e}")))?;
    let source_tenant = TenantId(
        Uuid::from_slice(&source_bytes)
            .map_err(|e| ArgoError::Provider(format!("source_tenant_id from db: {e}")))?,
    );
    let consumer_tenant = TenantId(
        Uuid::from_slice(&consumer_bytes)
            .map_err(|e| ArgoError::Provider(format!("consumer_tenant_id from db: {e}")))?,
    );
    let skill_id = SkillId(
        Uuid::from_slice(&skill_bytes)
            .map_err(|e| ArgoError::Provider(format!("skill_id from db: {e}")))?,
    );
    let observed_at = DateTime::<Utc>::from_timestamp_micros(observed_at_micros).ok_or_else(
        || ArgoError::Provider(format!("trust_observation observed_at out of range: {observed_at_micros}")),
    )?;
    Ok(TrustObservationRow {
        id,
        source_tenant,
        consumer_tenant,
        skill_id,
        quality_score: quality as f32,
        observed_at,
    })
}

#[cfg(test)]
#[path = "trust_observation_tests.rs"]
mod tests;
