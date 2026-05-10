//! Canary rollout for promoted skill versions.
//!
//! When the operator promotes a new skill version, full-traffic
//! cutover is risky: a regression on quality, refusal rate, or
//! latency only surfaces once everything is on fire. A canary
//! routes `traffic_percent` of invocations to the new version and
//! leaves the rest on baseline so the divergence can be observed
//! before going to 100%.
//!
//! ## Pieces
//!
//! - [`CanaryConfig`] — one row per skill family. Contains
//!   `baseline_version` + `canary_version` + `traffic_percent`
//!   + `started_at` + optional `expires_at` (NULL = no expiry).
//! - [`pick_version`] — deterministic selector. Same
//!   `(skill_id, invocation_seed)` always maps to the same
//!   version (seeded SHA-256 hash, no RNG), so retries don't
//!   shuffle assignment.
//! - [`CanaryStore`] — persistence facade backed by the
//!   `skill_canary` table (migration v43, raw SQL `47__skill_canary.sql`).
//! - [`resolve_active_body`] — runner-side integration helper.
//!   Given the baseline body and a stable per-invocation seed,
//!   either returns the baseline body unchanged or loads the
//!   canary version's body and emits a routing-decision audit
//!   row tagged `kind=canary_pick`.
//!
//! ## Audit
//!
//! [`pick_for_skill`] returns a [`CanaryDecision`] that the caller
//! (the agent runner) writes as an `AuditKind::RoutingDecision`
//! row tagged with `kind: "canary_pick"` in the JSON payload.
//! This module deliberately does NOT depend on `argo-audit` —
//! adding a dedicated `AuditKind::CanaryPick` variant or wiring
//! `AuditSink` here would require touching crates outside this
//! brief's may-touch list, so the audit-write responsibility lives
//! at the runner-side integration point.
use crate::store::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;

/// Operator-defined canary for one skill family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanaryConfig {
    /// Skill family identifier (the `skill.name` column — same
    /// label the operator types into `argo skill versions <name>`).
    pub skill_id: String,
    /// Version string of the version that should serve `100 -
    /// traffic_percent` of invocations (the "old, known-good" row).
    pub baseline_version: String,
    /// Version string of the canary candidate.
    pub canary_version: String,
    /// Percent of invocations that should land on `canary_version`.
    /// Clamped to `[0, 100]` at write time by the table CHECK
    /// constraint. `0` → always baseline; `100` → always canary.
    pub traffic_percent: u8,
    /// Microseconds since unix epoch when the canary was created.
    pub started_at: i64,
    /// Optional micros-since-epoch expiry. `None` = no expiry.
    /// Once `now > expires_at`, [`pick_for_skill`] returns `None`
    /// so callers stay on the baseline body.
    pub expires_at: Option<i64>,
}

/// `true` when `seed` should land on the canary side of a
/// `traffic_percent` split. Deterministic in `seed` — same input
/// always yields the same answer.
///
/// Edge cases: `0` always returns `false`, `100`+ always `true`,
/// values above `100` saturate to `true`. The intermediate range
/// uses `(sha256(seed) % 100) < percent` which gives a uniform
/// distribution over the seed space.
#[must_use]
pub fn is_canary(seed: u64, traffic_percent: u8) -> bool {
    if traffic_percent == 0 {
        return false;
    }
    if traffic_percent >= 100 {
        return true;
    }
    let h = hash_seed(seed);
    (h % 100) < u64::from(traffic_percent)
}

/// Pick the version string to use for an invocation. Borrows
/// from `cfg` so the caller can avoid an allocation when the
/// chosen version is then handed to a SQL bind.
#[must_use]
pub fn pick_version(cfg: &CanaryConfig, invocation_seed: u64) -> &str {
    if is_canary(invocation_seed, cfg.traffic_percent) {
        &cfg.canary_version
    } else {
        &cfg.baseline_version
    }
}

/// Stable seed → u64 hasher. SHA-256 is overkill for a 100-bucket
/// split but it's already a workspace dep and the choice is stable
/// across Rust toolchain versions (unlike `DefaultHasher`).
#[must_use]
pub fn hash_seed(seed: u64) -> u64 {
    let mut h = Sha256::new();
    h.update(seed.to_le_bytes());
    let digest = h.finalize();
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(buf)
}

/// Persistence facade for the `skill_canary` table.
#[derive(Clone)]
pub struct CanaryStore {
    store: Arc<SessionStore>,
}

impl std::fmt::Debug for CanaryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanaryStore").finish()
    }
}

impl CanaryStore {
    /// Build over an open session store.
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Upsert a canary config — replaces any prior config for the
    /// same `skill_id`. The PRIMARY KEY constraint enforces
    /// single-row-per-skill semantics.
    pub async fn set(&self, cfg: &CanaryConfig) -> ArgoResult<()> {
        if cfg.traffic_percent > 100 {
            return Err(ArgoError::Schema(format!(
                "canary: traffic_percent {} out of range (0..=100)",
                cfg.traffic_percent
            )));
        }
        if cfg.baseline_version == cfg.canary_version {
            return Err(ArgoError::Schema(
                "canary: baseline_version and canary_version must differ".into(),
            ));
        }
        sqlx::query(
            "INSERT INTO skill_canary \
             (skill_id, baseline_version, canary_version, traffic_percent, started_at, expires_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(skill_id) DO UPDATE SET \
                baseline_version = excluded.baseline_version, \
                canary_version   = excluded.canary_version, \
                traffic_percent  = excluded.traffic_percent, \
                started_at       = excluded.started_at, \
                expires_at       = excluded.expires_at",
        )
        .bind(&cfg.skill_id)
        .bind(&cfg.baseline_version)
        .bind(&cfg.canary_version)
        .bind(i64::from(cfg.traffic_percent))
        .bind(cfg.started_at)
        .bind(cfg.expires_at)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("canary set: {e}")))?;
        Ok(())
    }

    /// Delete the canary config for `skill_id`. Returns rows
    /// affected (0 when nothing was set).
    pub async fn clear(&self, skill_id: &str) -> ArgoResult<u64> {
        let res = sqlx::query("DELETE FROM skill_canary WHERE skill_id = ?")
            .bind(skill_id)
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("canary clear: {e}")))?;
        Ok(res.rows_affected())
    }

    /// Fetch the canary config for `skill_id`, or `None` if no
    /// canary is in flight for that skill.
    pub async fn get(&self, skill_id: &str) -> ArgoResult<Option<CanaryConfig>> {
        let row = sqlx::query(
            "SELECT skill_id, baseline_version, canary_version, traffic_percent, \
             started_at, expires_at FROM skill_canary WHERE skill_id = ?",
        )
        .bind(skill_id)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("canary get: {e}")))?;
        Ok(row.map(row_to_config))
    }

    /// List every canary config in this workspace, ordered by
    /// `skill_id`. Used by the (future) `argo skill canary list`
    /// CLI command.
    pub async fn list(&self) -> ArgoResult<Vec<CanaryConfig>> {
        let rows = sqlx::query(
            "SELECT skill_id, baseline_version, canary_version, traffic_percent, \
             started_at, expires_at FROM skill_canary ORDER BY skill_id ASC",
        )
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("canary list: {e}")))?;
        Ok(rows.into_iter().map(row_to_config).collect())
    }
}

fn row_to_config(row: sqlx::sqlite::SqliteRow) -> CanaryConfig {
    let percent: i64 = row.get("traffic_percent");
    let percent = percent.clamp(0, 100) as u8;
    CanaryConfig {
        skill_id: row.get("skill_id"),
        baseline_version: row.get("baseline_version"),
        canary_version: row.get("canary_version"),
        traffic_percent: percent,
        started_at: row.get("started_at"),
        expires_at: row.get("expires_at"),
    }
}

/// Per-invocation routing decision the runner consumes after
/// calling [`pick_for_skill`]. Carries everything the caller needs
/// to write the canary-pick audit row + decide whether to swap the
/// active skill body to the canary version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanaryDecision {
    /// Baseline version label from the active config.
    pub baseline_version: String,
    /// Canary version label from the active config.
    pub canary_version: String,
    /// Whichever one the seed selected (one of the two above).
    pub chosen_version: String,
    /// Echo of the active config's traffic_percent for audit analytics.
    pub traffic_percent: u8,
    /// Hashed seed (so audit rows don't leak raw session/skill seeds).
    pub seed_hash: u64,
    /// Convenience flag: `chosen_version == canary_version`.
    pub chose_canary: bool,
}

/// Decide which version of `skill_id` should serve this invocation.
///
/// Returns `Ok(None)` when no canary config exists for the skill
/// OR the config has expired (`expires_at <= now_micros`). The
/// caller then keeps the baseline body it already has and skips
/// audit writing. Returns `Ok(Some(decision))` for an active,
/// non-expired config; the caller writes the audit row and (if
/// `decision.chose_canary`) swaps the active body via [`load_body`].
///
/// `now_micros` is a parameter (not `Utc::now`) so tests pin time.
pub async fn pick_for_skill(
    canary: &CanaryStore,
    skill_id: &str,
    invocation_seed: u64,
    now_micros: i64,
) -> ArgoResult<Option<CanaryDecision>> {
    let Some(cfg) = canary.get(skill_id).await? else {
        return Ok(None);
    };
    if let Some(exp) = cfg.expires_at {
        if exp <= now_micros {
            return Ok(None);
        }
    }
    let chosen = pick_version(&cfg, invocation_seed).to_string();
    let chose_canary = chosen == cfg.canary_version;
    Ok(Some(CanaryDecision {
        baseline_version: cfg.baseline_version,
        canary_version: cfg.canary_version,
        chosen_version: chosen,
        traffic_percent: cfg.traffic_percent,
        seed_hash: hash_seed(invocation_seed),
        chose_canary,
    }))
}

/// Load a skill body by `(name, version)`. Returns `Ok(None)` when
/// no such version row exists — the runner converts that to a hard
/// error (a canary configured against a missing version is a
/// promotion bug).
pub async fn load_body(
    skills: &SkillStore,
    name: &str,
    version: &str,
) -> ArgoResult<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT body FROM skill WHERE name = ? AND version = ?")
            .bind(name)
            .bind(version)
            .fetch_optional(skills.store_arc().pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("canary load body: {e}")))?;
    Ok(row.map(|(b,)| b))
}

/// Convenience seed: hash `(session_id, skill_id)` so a session
/// keeps the same canary decision for a given skill across all
/// turns within that session (no flip-flopping mid-conversation).
#[must_use]
pub fn seed_for_session_skill(session_id: SessionId, skill_id: &str) -> u64 {
    let mut h = Sha256::new();
    h.update(session_id.0.as_bytes());
    h.update(b"::");
    h.update(skill_id.as_bytes());
    let digest = h.finalize();
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(buf)
}

#[cfg(test)]
#[path = "canary_tests.rs"]
mod canary_tests;

/// Build a `CanaryConfig` with `started_at = now`, no expiry.
/// Validates traffic_percent ∈ [0,100] and that the two versions
/// differ — same checks as [`CanaryStore::set`], surfaced early so
/// callers can fail fast before touching the DB.
pub fn new_config(
    skill_id: impl Into<String>,
    baseline_version: impl Into<String>,
    canary_version: impl Into<String>,
    traffic_percent: u8,
) -> ArgoResult<CanaryConfig> {
    if traffic_percent > 100 {
        return Err(ArgoError::Schema(format!(
            "canary: traffic_percent {traffic_percent} out of range (0..=100)"
        )));
    }
    let baseline_version = baseline_version.into();
    let canary_version = canary_version.into();
    if baseline_version == canary_version {
        return Err(ArgoError::Schema(
            "canary: baseline_version and canary_version must differ".into(),
        ));
    }
    Ok(CanaryConfig {
        skill_id: skill_id.into(),
        baseline_version,
        canary_version,
        traffic_percent,
        started_at: Utc::now().timestamp_micros(),
        expires_at: None,
    })
}

