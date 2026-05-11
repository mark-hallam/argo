//! Skill version migration records — the publisher-side declaration
//! that "v2 supersedes v1; here's how to upgrade." Read by the
//! `argo skill upgrade` tenant resolver, the `argo skill
//! list-migrations` operator dashboard, and the agent session-boot
//! hint that fires `tracing::info!` per pending migration.
//!
//! Backed by the `skill_version_migration` table (migration v50).
//! Composite PK `(skill_id, from_version, to_version)` makes
//! [`record_migration`] idempotent on retry without an explicit
//! existence check; the secondary index
//! `(skill_id, to_version)` covers the "what migrations land me at
//! v2?" lookup pattern.
//!
//! Brief: `mission-skill-version-migration-path` (MISSION_BACKLOG
//! Section 3 Claim 4 #79).
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId, TenantId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// Maximum length for the operator-supplied `reason` text. Longer
/// strings are truncated with an ellipsis on insert; the brief calls
/// for a "reason-text length cap" test, so this is enforced once at
/// record time rather than at every read site.
pub const MIGRATION_REASON_MAX_LEN: usize = 500;

/// One row from `skill_version_migration` projected into a stable
/// public shape for CLI / JSON callers. Blob ids are hex-encoded for
/// consistency with [`crate::version::SkillVersion`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationRow {
    /// Hex-encoded UUID of the skill family this migration applies
    /// to (matches `skill.id`'s blob, encoded with the same
    /// lowercase-no-dashes convention as `SkillVersion.id`).
    pub skill_id: String,
    /// Version string the migration applies FROM (the installed
    /// version on tenant workspaces that should upgrade).
    pub from_version: String,
    /// Version string the migration upgrades TO.
    pub to_version: String,
    /// Hex-encoded tenant id of the publisher who recorded the
    /// migration. Surfaced for operator audits and future
    /// publisher-ownership enforcement (TODO: gate
    /// [`record_migration`] on `publisher_tenant_id ==
    /// skill.publisher_tenant_id`).
    pub publisher_tenant_id: String,
    /// Optional free-text rationale ("v2 fixes the trailing-slash
    /// bug"); capped to [`MIGRATION_REASON_MAX_LEN`] chars on
    /// insert.
    pub reason: Option<String>,
    /// Whether `argo skill upgrade <id>` should auto-apply without
    /// `--yes`.
    pub auto_upgrade: bool,
    /// Micros since the epoch the row was recorded at.
    pub created_at_micros: i64,
}

/// Insert (or no-op) a publisher-side migration row.
///
/// Idempotent on the composite PK `(skill_id, from_version,
/// to_version)` — re-running a CI job that records the same
/// migration twice does not error and does not bump `created_at`.
/// The `reason` is truncated to [`MIGRATION_REASON_MAX_LEN`] on
/// insert.
pub async fn record_migration(
    store: &SessionStore,
    skill_id: SkillId,
    from_version: &str,
    to_version: &str,
    publisher_tenant_id: TenantId,
    reason: Option<&str>,
    auto_upgrade: bool,
) -> ArgoResult<()> {
    let trimmed_reason = reason.map(truncate_reason);
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT OR IGNORE INTO skill_version_migration \
         (skill_id, from_version, to_version, publisher_tenant_id, \
          reason, auto_upgrade, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(skill_id.0.as_bytes().as_slice())
    .bind(from_version)
    .bind(to_version)
    .bind(publisher_tenant_id.0.as_bytes().as_slice())
    .bind(trimmed_reason)
    .bind(i64::from(auto_upgrade))
    .bind(now)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("record_migration: {e}")))?;
    Ok(())
}

/// Find a migration whose `from_version` matches `installed_version`
/// for the given skill family. Returns `Ok(None)` when no migration
/// is pending. Uses the composite PK's `(skill_id, from_version)`
/// prefix index — no table scan.
///
/// If multiple `to_version` rows exist for one `from_version` (e.g.
/// `v1 -> v2` and `v1 -> v3`), the newest-by-`created_at` row wins.
/// Operators can list all candidates via [`list_for_skill`].
pub async fn lookup_migration(
    store: &SessionStore,
    skill_id: SkillId,
    installed_version: &str,
) -> ArgoResult<Option<MigrationRow>> {
    let row = sqlx::query(
        "SELECT skill_id, from_version, to_version, publisher_tenant_id, \
         reason, auto_upgrade, created_at \
         FROM skill_version_migration \
         WHERE skill_id = ? AND from_version = ? \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(skill_id.0.as_bytes().as_slice())
    .bind(installed_version)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("lookup_migration: {e}")))?;
    Ok(row.map(decode_row))
}

/// List every recorded migration for one skill family, newest first.
/// Operator visibility surface for `argo skill list-migrations` and
/// for diagnosing "why did v3 not pick up the v2->v3 row?" cases.
pub async fn list_for_skill(
    store: &SessionStore,
    skill_id: SkillId,
) -> ArgoResult<Vec<MigrationRow>> {
    let rows = sqlx::query(
        "SELECT skill_id, from_version, to_version, publisher_tenant_id, \
         reason, auto_upgrade, created_at \
         FROM skill_version_migration \
         WHERE skill_id = ? \
         ORDER BY created_at DESC, from_version, to_version",
    )
    .bind(skill_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list_for_skill: {e}")))?;
    Ok(rows.into_iter().map(decode_row).collect())
}

fn decode_row(row: sqlx::sqlite::SqliteRow) -> MigrationRow {
    let skill_id_blob: Vec<u8> = row.get("skill_id");
    let publisher_blob: Vec<u8> = row.get("publisher_tenant_id");
    let auto_upgrade_int: i64 = row.get("auto_upgrade");
    MigrationRow {
        skill_id: hex_from_blob(&skill_id_blob),
        from_version: row.get("from_version"),
        to_version: row.get("to_version"),
        publisher_tenant_id: hex_from_blob(&publisher_blob),
        reason: row.get::<Option<String>, _>("reason"),
        auto_upgrade: auto_upgrade_int != 0,
        created_at_micros: row.get("created_at"),
    }
}

fn hex_from_blob(blob: &[u8]) -> String {
    blob.iter().map(|b| format!("{b:02x}")).collect()
}

fn truncate_reason(reason: &str) -> String {
    if reason.chars().count() <= MIGRATION_REASON_MAX_LEN {
        return reason.to_string();
    }
    let mut out: String = reason.chars().take(MIGRATION_REASON_MAX_LEN - 1).collect();
    out.push('…');
    out
}

/// Decode a hex-encoded skill id (32 lowercase nibbles, no dashes —
/// the format produced by [`MigrationRow::skill_id`]) back into a
/// [`SkillId`]. CLI handlers accept either this form or the
/// dashed-UUID form via `Uuid::parse_str`.
pub fn parse_skill_id_hex(hex: &str) -> ArgoResult<SkillId> {
    if hex.len() == 32 {
        let mut bytes = [0u8; 16];
        for (i, byte) in bytes.iter_mut().enumerate() {
            let pair = &hex[i * 2..i * 2 + 2];
            *byte = u8::from_str_radix(pair, 16)
                .map_err(|e| ArgoError::Schema(format!("skill id hex: {e}")))?;
        }
        return Ok(SkillId(Uuid::from_bytes(bytes)));
    }
    let parsed = Uuid::parse_str(hex)
        .map_err(|e| ArgoError::Schema(format!("skill id parse: {e}")))?;
    Ok(SkillId(parsed))
}

#[cfg(test)]
#[path = "version_migration_tests.rs"]
mod tests;
