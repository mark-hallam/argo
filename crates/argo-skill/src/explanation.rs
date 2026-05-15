//! Skill explanation envelope: provenance, versions, forge history,
//! trust observations, and recent invocation summary.

use crate::{SkillStore, TrustGrade};
use argo_store::skill_provenance::{query_for_skill, SkillProvenanceKind, SkillProvenanceRow};
use argo_store::{runs_for_skill, SessionStore};
use argo_types::{ArgoError, ArgoResult};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Full `argo skill explain` envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillExplanation {
    /// Active skill name.
    pub name: String,
    /// Active version label.
    pub current_version: String,
    /// Trust grade derived from the active skill's provenance source.
    pub current_trust_grade: TrustGrade,
    /// How the active version entered the system.
    pub provenance: SkillProvenance,
    /// Bounded version history, oldest to newest.
    pub version_history: Vec<SkillVersionEntry>,
    /// Bounded Forge run history, newest first.
    pub forge_runs: Vec<ForgeRunSummary>,
    /// Dreams lineage when the current provenance is Dreams-derived.
    pub dreams_lineage: Option<DreamsLineage>,
    /// Bounded trust observations, newest first.
    pub trust_history: Vec<TrustObservationSummary>,
    /// Recent invocation aggregate.
    pub usage_summary: UsageSummary,
}

/// Provenance of one skill version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillProvenance {
    /// Shipped with the binary or vertical bundle.
    BootstrapShipped {
        /// Timestamp in epoch microseconds.
        ts_micros: i64,
    },
    /// Authored by an operator.
    OperatorAuthored {
        /// Actor id/name when known.
        actor: Option<String>,
        /// Timestamp in epoch microseconds.
        ts_micros: i64,
    },
    /// Derived from a Dreams template/cohort.
    DreamsDerived {
        /// Parent template id when known.
        parent_template_id: Option<String>,
        /// Timestamp in epoch microseconds.
        ts_micros: i64,
    },
    /// Promoted from a Forge-generated version.
    ForgePromoted {
        /// Prior/base version when known.
        from_version: Option<String>,
        /// Timestamp in epoch microseconds.
        ts_micros: i64,
    },
    /// Imported from an external artifact or marketplace source.
    Imported {
        /// Source URL/id when known.
        source_url: Option<String>,
        /// Timestamp in epoch microseconds.
        ts_micros: i64,
    },
}

/// One version row in the explanation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillVersionEntry {
    /// Version label.
    pub version: String,
    /// Created timestamp, epoch microseconds.
    pub ts_micros: i64,
    /// One-line reason.
    pub change_reason: Option<String>,
    /// Actor id/name when known.
    pub author: Option<String>,
    /// Trust grade at that version.
    pub trust_grade_at_version: TrustGrade,
    /// Whether this version contains a breaking change.
    pub breaking_change: bool,
}

/// One Forge run summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForgeRunSummary {
    /// Run id.
    pub run_id: String,
    /// Start timestamp, epoch microseconds.
    pub ts_micros: i64,
    /// Final judge/quality score when known.
    pub judge_score: Option<f64>,
    /// Iterations/mutations applied.
    pub mutations_applied: u32,
    /// Whether the run succeeded/partially succeeded.
    pub accepted: bool,
}

/// Dreams lineage for a Dreams-derived skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DreamsLineage {
    /// Parent template id when known.
    pub parent_template_id: Option<String>,
    /// Number of trajectories in the cohort when known.
    pub trajectory_count: u32,
    /// Promotion judge score when known.
    pub promotion_judge_score: Option<f64>,
}

/// One trust observation summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustObservationSummary {
    /// Source tenant UUID string.
    pub source_tenant: String,
    /// Consumer tenant UUID string.
    pub consumer_tenant: String,
    /// Quality score in `[0, 1]`.
    pub quality_score: f32,
    /// Observation timestamp, epoch microseconds.
    pub observed_at_micros: i64,
}

/// Recent skill invocation aggregate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageSummary {
    /// Successful invocations in the last 7 days.
    pub invocations_last_7d: u32,
    /// All invocations in the last 30 days.
    pub invocations_last_30d: u32,
    /// Median latency over rows that recorded latency.
    pub p50_latency_ms: Option<i64>,
    /// P99 latency over rows that recorded latency.
    pub p99_latency_ms: Option<i64>,
    /// Failed/skipped percentage over the last 30 days.
    pub error_rate_pct: f32,
}

/// Compose a [`SkillExplanation`] for `name`.
pub async fn explain_skill(
    skills: &SkillStore,
    name: &str,
    history_depth: usize,
) -> ArgoResult<SkillExplanation> {
    let depth = history_depth.clamp(1, 100);
    let active = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::NotFound(format!("skill '{name}'")))?;
    let versions = skills.versions_for_name(name).await?;
    let store = skills.store_arc();
    let provenance_rows = query_for_skill(store.pool(), name, depth as u32).await?;
    let current_provenance = provenance_rows
        .iter()
        .find(|row| row.skill_version == active.frontmatter.version)
        .cloned()
        .unwrap_or_else(|| fallback_row(&active));
    let version_history = build_version_history(&versions, &provenance_rows, depth);
    let forge_runs = query_forge_runs(&store, name, depth).await?;
    let trust_history = query_trust_history(&store, &active.id.0.to_string(), depth).await?;
    let usage_summary = build_usage_summary(&store, name).await?;
    let dreams_lineage = match current_provenance.kind {
        SkillProvenanceKind::DreamsDerived => Some(DreamsLineage {
            parent_template_id: current_provenance.parent_id.clone(),
            trajectory_count: 0,
            promotion_judge_score: None,
        }),
        _ => None,
    };
    Ok(SkillExplanation {
        name: active.frontmatter.name.clone(),
        current_version: active.frontmatter.version.clone(),
        current_trust_grade: TrustGrade::from_source(&active.frontmatter.provenance.source),
        provenance: provenance_from_row(current_provenance),
        version_history,
        forge_runs,
        dreams_lineage,
        trust_history,
        usage_summary,
    })
}

fn build_version_history(
    versions: &[crate::Skill],
    provenance_rows: &[SkillProvenanceRow],
    depth: usize,
) -> Vec<SkillVersionEntry> {
    let start = versions.len().saturating_sub(depth);
    versions[start..]
        .iter()
        .map(|skill| {
            let row = provenance_rows
                .iter()
                .find(|row| row.skill_version == skill.frontmatter.version);
            SkillVersionEntry {
                version: skill.frontmatter.version.clone(),
                ts_micros: row
                    .map(|row| row.ts_micros)
                    .unwrap_or_else(|| skill.created_at.timestamp_micros()),
                change_reason: row.and_then(|row| row.change_reason.clone()),
                author: row.and_then(|row| row.actor.clone()),
                trust_grade_at_version: TrustGrade::from_source(
                    &skill.frontmatter.provenance.source,
                ),
                breaking_change: row.map(|row| row.breaking_change).unwrap_or(false),
            }
        })
        .collect()
}

async fn query_forge_runs(
    store: &SessionStore,
    name: &str,
    depth: usize,
) -> ArgoResult<Vec<ForgeRunSummary>> {
    let rows = sqlx::query(
        "SELECT id, started_at, iterations, final_score, status \
         FROM forge_run WHERE skill_name = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(name)
    .bind(i64::try_from(depth).unwrap_or(100))
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill explain forge runs: {e}")))?;
    rows.into_iter()
        .map(|row| {
            let id: Vec<u8> = row.try_get("id").map_err(decode_err)?;
            let run_id = uuid::Uuid::from_slice(&id)
                .map_err(|e| ArgoError::Schema(format!("forge_run.id: {e}")))?
                .to_string();
            let iterations: i64 = row.try_get("iterations").unwrap_or(0);
            let status: String = row.try_get("status").unwrap_or_default();
            Ok(ForgeRunSummary {
                run_id,
                ts_micros: row.try_get("started_at").map_err(decode_err)?,
                judge_score: row.try_get("final_score").ok(),
                mutations_applied: u32::try_from(iterations.max(0)).unwrap_or(u32::MAX),
                accepted: matches!(status.as_str(), "succeeded" | "partial" | "completed"),
            })
        })
        .collect()
}

async fn query_trust_history(
    store: &SessionStore,
    skill_id: &str,
    depth: usize,
) -> ArgoResult<Vec<TrustObservationSummary>> {
    let rows = sqlx::query(
        "SELECT source_tenant_id, consumer_tenant_id, quality_score, observed_at \
         FROM trust_observation WHERE lower(hex(skill_id)) = ? \
         ORDER BY observed_at DESC LIMIT ?",
    )
    .bind(skill_id.replace('-', "").to_lowercase())
    .bind(i64::try_from(depth).unwrap_or(100))
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill explain trust history: {e}")))?;
    rows.into_iter()
        .map(|row| {
            let source: Vec<u8> = row.try_get("source_tenant_id").map_err(decode_err)?;
            let consumer: Vec<u8> = row.try_get("consumer_tenant_id").map_err(decode_err)?;
            Ok(TrustObservationSummary {
                source_tenant: uuid::Uuid::from_slice(&source)
                    .map_err(|e| ArgoError::Schema(format!("source_tenant_id: {e}")))?
                    .to_string(),
                consumer_tenant: uuid::Uuid::from_slice(&consumer)
                    .map_err(|e| ArgoError::Schema(format!("consumer_tenant_id: {e}")))?
                    .to_string(),
                quality_score: row.try_get::<f64, _>("quality_score").unwrap_or(0.0) as f32,
                observed_at_micros: row.try_get("observed_at").map_err(decode_err)?,
            })
        })
        .collect()
}

async fn build_usage_summary(store: &SessionStore, name: &str) -> ArgoResult<UsageSummary> {
    let now = Utc::now();
    let since_30 = (now - Duration::days(30)).timestamp_micros();
    let since_7 = (now - Duration::days(7)).timestamp_micros();
    let rows = runs_for_skill(store, name, None, Some(since_30), 1000, true).await?;
    let invocations_last_30d = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    let invocations_last_7d = u32::try_from(
        rows.iter()
            .filter(|row| row.timestamp_micros >= since_7 && row.decision == "success")
            .count(),
    )
    .unwrap_or(u32::MAX);
    let failures = rows.iter().filter(|row| row.decision != "success").count();
    let mut latencies: Vec<i64> = rows.iter().filter_map(|row| row.latency_ms).collect();
    latencies.sort_unstable();
    let error_rate_pct = if rows.is_empty() {
        0.0
    } else {
        (failures as f32 / rows.len() as f32) * 100.0
    };
    Ok(UsageSummary {
        invocations_last_7d,
        invocations_last_30d,
        p50_latency_ms: percentile(&latencies, 50),
        p99_latency_ms: percentile(&latencies, 99),
        error_rate_pct,
    })
}

fn percentile(values: &[i64], pct: usize) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let idx = ((values.len() - 1) * pct) / 100;
    values.get(idx).copied()
}

fn fallback_row(skill: &crate::Skill) -> SkillProvenanceRow {
    SkillProvenanceRow {
        skill_name: skill.frontmatter.name.clone(),
        skill_version: skill.frontmatter.version.clone(),
        kind: kind_from_source(&skill.frontmatter.provenance.source),
        actor: None,
        parent_id: None,
        change_reason: Some("derived from skill frontmatter".into()),
        breaking_change: false,
        ts_micros: skill.created_at.timestamp_micros(),
    }
}

fn kind_from_source(source: &str) -> SkillProvenanceKind {
    if source == "builtin" || source.starts_with("bundle:") {
        SkillProvenanceKind::Bootstrap
    } else if source == "user"
        || source == "forge"
        || source == "cli-create"
        || source.starts_with("refined-from-")
    {
        SkillProvenanceKind::OperatorAuthored
    } else if source == "learned" || source.starts_with("learned:") {
        SkillProvenanceKind::DreamsDerived
    } else {
        SkillProvenanceKind::Imported
    }
}

fn provenance_from_row(row: SkillProvenanceRow) -> SkillProvenance {
    match row.kind {
        SkillProvenanceKind::Bootstrap => SkillProvenance::BootstrapShipped {
            ts_micros: row.ts_micros,
        },
        SkillProvenanceKind::OperatorAuthored => SkillProvenance::OperatorAuthored {
            actor: row.actor,
            ts_micros: row.ts_micros,
        },
        SkillProvenanceKind::DreamsDerived => SkillProvenance::DreamsDerived {
            parent_template_id: row.parent_id,
            ts_micros: row.ts_micros,
        },
        SkillProvenanceKind::ForgePromoted => SkillProvenance::ForgePromoted {
            from_version: row.parent_id,
            ts_micros: row.ts_micros,
        },
        SkillProvenanceKind::Imported => SkillProvenance::Imported {
            source_url: row.parent_id,
            ts_micros: row.ts_micros,
        },
    }
}

fn decode_err(e: sqlx::Error) -> ArgoError {
    ArgoError::Provider(format!("skill explain decode: {e}"))
}

#[cfg(test)]
#[path = "explanation_tests.rs"]
mod tests;
