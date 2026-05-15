//! Append-only skill provenance log backing `argo skill explain`.
//!
//! One row is stored per `(skill_name, skill_version)`. Callers may
//! upsert because older workspaces are backfilled as `Bootstrap` first
//! and later mutation paths can replace that row with a more precise
//! provenance kind.

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};

/// Stable provenance kind stored in `skill_provenance.provenance_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillProvenanceKind {
    /// Shipped with the binary or a bundled vertical.
    Bootstrap,
    /// Authored directly by an operator in this workspace.
    OperatorAuthored,
    /// Derived from a Dreams cohort/template.
    DreamsDerived,
    /// Promoted from a Forge optimization run.
    ForgePromoted,
    /// Imported from an external marketplace/artifact.
    Imported,
}

impl SkillProvenanceKind {
    /// SQLite label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bootstrap => "Bootstrap",
            Self::OperatorAuthored => "OperatorAuthored",
            Self::DreamsDerived => "DreamsDerived",
            Self::ForgePromoted => "ForgePromoted",
            Self::Imported => "Imported",
        }
    }

    /// Parse a SQLite label.
    pub fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "Bootstrap" => Ok(Self::Bootstrap),
            "OperatorAuthored" => Ok(Self::OperatorAuthored),
            "DreamsDerived" => Ok(Self::DreamsDerived),
            "ForgePromoted" => Ok(Self::ForgePromoted),
            "Imported" => Ok(Self::Imported),
            other => Err(ArgoError::Schema(format!(
                "unknown skill provenance kind '{other}'"
            ))),
        }
    }
}

/// Insert payload for [`record_provenance`].
#[derive(Debug, Clone)]
pub struct NewSkillProvenance<'a> {
    /// Skill name.
    pub skill_name: &'a str,
    /// Skill version label.
    pub skill_version: &'a str,
    /// Provenance kind.
    pub kind: SkillProvenanceKind,
    /// Operator or system actor, if known.
    pub actor: Option<&'a str>,
    /// Parent template/source/prior-version id, depending on kind.
    pub parent_id: Option<&'a str>,
    /// Human-readable reason.
    pub change_reason: Option<&'a str>,
    /// Whether this version contains a breaking change.
    pub breaking_change: bool,
}

/// One persisted provenance row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProvenanceRow {
    /// Skill name.
    pub skill_name: String,
    /// Skill version label.
    pub skill_version: String,
    /// Provenance kind.
    pub kind: SkillProvenanceKind,
    /// Operator or system actor, if known.
    pub actor: Option<String>,
    /// Parent template/source/prior-version id, depending on kind.
    pub parent_id: Option<String>,
    /// Human-readable reason.
    pub change_reason: Option<String>,
    /// Whether this version contains a breaking change.
    pub breaking_change: bool,
    /// Creation/update timestamp, epoch microseconds.
    pub ts_micros: i64,
}

/// Insert or replace one provenance row.
pub async fn record_provenance(pool: &Pool<Sqlite>, new: NewSkillProvenance<'_>) -> ArgoResult<()> {
    let reason = new.change_reason.map(truncate_reason);
    sqlx::query(
        "INSERT INTO skill_provenance \
            (skill_name, skill_version, provenance_kind, actor, parent_id, \
             change_reason, breaking_change, ts) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(skill_name, skill_version) DO UPDATE SET \
            provenance_kind = excluded.provenance_kind, \
            actor = excluded.actor, \
            parent_id = excluded.parent_id, \
            change_reason = excluded.change_reason, \
            breaking_change = excluded.breaking_change, \
            ts = excluded.ts",
    )
    .bind(new.skill_name)
    .bind(new.skill_version)
    .bind(new.kind.as_str())
    .bind(new.actor)
    .bind(new.parent_id)
    .bind(reason.as_deref())
    .bind(if new.breaking_change { 1_i64 } else { 0_i64 })
    .bind(chrono::Utc::now().timestamp_micros())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_provenance insert: {e}")))?;
    Ok(())
}

/// Return the provenance rows for a skill, newest first.
pub async fn query_for_skill(
    pool: &Pool<Sqlite>,
    skill_name: &str,
    limit: u32,
) -> ArgoResult<Vec<SkillProvenanceRow>> {
    let rows = sqlx::query(
        "SELECT skill_name, skill_version, provenance_kind, actor, parent_id, \
                change_reason, breaking_change, ts \
         FROM skill_provenance \
         WHERE skill_name = ? ORDER BY ts DESC LIMIT ?",
    )
    .bind(skill_name)
    .bind(i64::from(limit.clamp(1, 1000)))
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_provenance query: {e}")))?;
    rows.into_iter().map(decode_row).collect()
}

fn decode_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<SkillProvenanceRow> {
    let kind_raw: String = row
        .try_get("provenance_kind")
        .map_err(|e| ArgoError::Provider(format!("skill_provenance kind: {e}")))?;
    let breaking: i64 = row.try_get("breaking_change").unwrap_or(0);
    Ok(SkillProvenanceRow {
        skill_name: row
            .try_get("skill_name")
            .map_err(|e| ArgoError::Provider(format!("skill_provenance name: {e}")))?,
        skill_version: row
            .try_get("skill_version")
            .map_err(|e| ArgoError::Provider(format!("skill_provenance version: {e}")))?,
        kind: SkillProvenanceKind::parse(&kind_raw)?,
        actor: row.try_get("actor").ok(),
        parent_id: row.try_get("parent_id").ok(),
        change_reason: row.try_get("change_reason").ok(),
        breaking_change: breaking != 0,
        ts_micros: row
            .try_get("ts")
            .map_err(|e| ArgoError::Provider(format!("skill_provenance ts: {e}")))?,
    })
}

fn truncate_reason(raw: &str) -> String {
    raw.chars().take(200).collect()
}

#[cfg(test)]
#[path = "skill_provenance_tests.rs"]
mod tests;
