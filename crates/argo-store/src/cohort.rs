//! Fleet-wide skill-cohort aggregation (Stream
//! `mission-skill-cohort-ui`, MISSION_BACKLOG #198).
//!
//! A **cohort** is the natural grouping `(vertical_id, trust_grade_band)`.
//! `trust_grade_band` buckets the integer `worker.trust_grade` so
//! adjacent grades land in the same operational tier.
//!
//! Two surface queries:
//!
//! * [`list_cohorts`] — one row per cohort with `member_count` and
//!   `skill_count` aggregates. Drives the `/cohorts` card grid.
//! * [`cohort_detail`] — the cohort summary plus per-member worker
//!   summaries and per-skill coverage percentages. Drives the detail
//!   drawer + coverage matrix.
//!
//! Read-only by design. `worker_skill.skill_id` is TEXT and holds a
//! skill *name* per existing convention; when a matching `skill` row
//! exists by name we surface its newest `version` lexicographically,
//! otherwise version is `None`.

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Hard cap on cohorts returned in one `list_cohorts` call.
pub const MAX_COHORTS: usize = 200;

/// Hard cap on member workers returned per cohort detail.
pub const MAX_COHORT_MEMBERS: usize = 200;

/// One row in the cohort list — drives the `/cohorts` card grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CohortSummary {
    /// Stable wire identifier — `"<vertical>::<trust_band>"`.
    pub cohort_id: String,
    /// `worker.vertical_id` value common to every member.
    pub vertical: String,
    /// Coarse trust band label (e.g. `"developing"`, `"trusted"`).
    pub trust_grade_band: String,
    /// Inclusive trust-grade lower bound of the band.
    pub trust_grade_min: u8,
    /// Inclusive trust-grade upper bound of the band.
    pub trust_grade_max: u8,
    /// Number of workers in the cohort.
    pub member_count: u32,
    /// Number of *distinct* `worker_skill.skill_id` values.
    pub skill_count: u32,
}

/// Single member row inside a [`CohortDetail`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CohortMember {
    /// `worker.id` as TEXT.
    pub worker_id: String,
    /// Integer trust grade stored on the worker row.
    pub trust_grade: u8,
    /// Skill identifiers held by this member, sorted ascending.
    pub skill_ids: Vec<String>,
}

/// Per-skill coverage row inside a [`CohortDetail`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkillCoverage {
    /// Canonical skill identifier (`worker_skill.skill_id`).
    pub skill_name: String,
    /// Newest version in `skill` matching this name, or `None`.
    pub version: Option<String>,
    /// Holders divided by total members, rounded to one decimal.
    pub member_coverage_pct: f64,
    /// Number of members holding the skill.
    pub member_hits: u32,
}

/// Output of [`cohort_detail`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CohortDetail {
    /// Cohort summary fields, flattened into the JSON object.
    #[serde(flatten)]
    pub summary: CohortSummary,
    /// One entry per cohort member, capped at [`MAX_COHORT_MEMBERS`].
    pub members: Vec<CohortMember>,
    /// One entry per distinct skill, sorted by coverage DESC then name.
    pub skills: Vec<SkillCoverage>,
}

/// `(vertical, grade_lo, grade_hi, band_label)`.
type BucketKey = (String, u8, u8, &'static str);
/// `(member_count, distinct skill ids)`.
type BucketValue = (u32, BTreeSet<String>);

const BANDS: &[(u8, u8, &str)] = &[
    (0, 0, "unknown"),
    (1, 3, "developing"),
    (4, 6, "established"),
    (7, 10, "trusted"),
    (11, u8::MAX, "principal"),
];

fn band_for(grade: u8) -> (u8, u8, &'static str) {
    for &(lo, hi, label) in BANDS {
        if grade >= lo && grade <= hi {
            return (lo, hi, label);
        }
    }
    (0, 0, "unknown")
}

fn cohort_id(vertical: &str, band_label: &str) -> String {
    format!("{vertical}::{band_label}")
}

/// Enumerate cohorts visible to `tenant_id`, sorted by `member_count
/// DESC, vertical ASC, trust_grade_min ASC`.
///
/// # Examples
///
/// ```no_run
/// # async fn demo(store: &argo_store::SessionStore, tenant: argo_types::TenantId)
/// #   -> argo_store::StoreResult<()> {
/// let cohorts = argo_store::cohort::list_cohorts(store, tenant).await?;
/// for c in cohorts {
///     println!("{} — {} members", c.cohort_id, c.member_count);
/// }
/// # Ok(()) }
/// ```
pub async fn list_cohorts(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<CohortSummary>> {
    let workers = fetch_workers(store, tenant_id).await?;
    let worker_skills = fetch_worker_skills(store, &worker_ids(&workers)).await?;
    let mut buckets: BTreeMap<BucketKey, BucketValue> = BTreeMap::new();
    for (worker_id, vertical, grade) in &workers {
        let (lo, hi, label) = band_for(*grade);
        let entry = buckets
            .entry((vertical.clone(), lo, hi, label))
            .or_insert_with(|| (0, BTreeSet::new()));
        entry.0 = entry.0.saturating_add(1);
        if let Some(skills) = worker_skills.get(worker_id) {
            for s in skills {
                entry.1.insert(s.clone());
            }
        }
    }
    let mut out: Vec<CohortSummary> = buckets
        .into_iter()
        .map(|((vertical, lo, hi, label), (member_count, skills))| CohortSummary {
            cohort_id: cohort_id(&vertical, label),
            vertical,
            trust_grade_band: label.to_string(),
            trust_grade_min: lo,
            trust_grade_max: hi,
            member_count,
            skill_count: u32::try_from(skills.len()).unwrap_or(u32::MAX),
        })
        .collect();
    out.sort_by(|a, b| {
        b.member_count
            .cmp(&a.member_count)
            .then_with(|| a.vertical.cmp(&b.vertical))
            .then_with(|| a.trust_grade_min.cmp(&b.trust_grade_min))
    });
    out.truncate(MAX_COHORTS);
    Ok(out)
}

/// Return one cohort's full breakdown — members + skill coverage.
///
/// Returns `Ok(None)` when no workers match `cohort_id`.
///
/// # Examples
///
/// ```no_run
/// # async fn demo(store: &argo_store::SessionStore, tenant: argo_types::TenantId)
/// #   -> argo_store::StoreResult<()> {
/// if let Some(detail) = argo_store::cohort::cohort_detail(store, tenant, "support::trusted").await? {
///     println!("{} skills", detail.skills.len());
/// }
/// # Ok(()) }
/// ```
pub async fn cohort_detail(
    store: &SessionStore,
    tenant_id: TenantId,
    cohort_id: &str,
) -> ArgoResult<Option<CohortDetail>> {
    let Some((vertical, band_label, lo, hi)) = parse_cohort_id(cohort_id) else {
        return Ok(None);
    };
    let workers = fetch_workers(store, tenant_id).await?;
    let matched: Vec<(String, u8)> = workers
        .into_iter()
        .filter(|(_, w_vertical, grade)| {
            w_vertical == &vertical && *grade >= lo && *grade <= hi
        })
        .map(|(id, _, grade)| (id, grade))
        .collect();
    if matched.is_empty() {
        return Ok(None);
    }
    let member_ids: Vec<String> = matched.iter().map(|(id, _)| id.clone()).collect();
    let worker_skills = fetch_worker_skills(store, &member_ids).await?;
    let member_count = u32::try_from(matched.len()).unwrap_or(u32::MAX);
    let truncated: Vec<(String, u8)> = matched.into_iter().take(MAX_COHORT_MEMBERS).collect();

    let mut skill_hits: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (worker_id, _) in &truncated {
        if let Some(skills) = worker_skills.get(worker_id) {
            for s in skills {
                skill_hits
                    .entry(s.clone())
                    .or_default()
                    .insert(worker_id.clone());
            }
        }
    }
    let distinct: Vec<String> = skill_hits.keys().cloned().collect();
    let latest_versions = latest_versions_for(store, &distinct).await?;
    let skill_count = u32::try_from(distinct.len()).unwrap_or(u32::MAX);

    let mut members: Vec<CohortMember> = truncated
        .into_iter()
        .map(|(worker_id, trust_grade)| {
            let mut skill_ids: Vec<String> =
                worker_skills.get(&worker_id).cloned().unwrap_or_default();
            skill_ids.sort();
            CohortMember {
                worker_id,
                trust_grade,
                skill_ids,
            }
        })
        .collect();
    members.sort_by(|a, b| {
        b.trust_grade
            .cmp(&a.trust_grade)
            .then_with(|| a.worker_id.cmp(&b.worker_id))
    });

    let denom = f64::from(member_count.max(1));
    let mut skills: Vec<SkillCoverage> = skill_hits
        .into_iter()
        .map(|(skill_name, holders)| {
            let hits = u32::try_from(holders.len()).unwrap_or(u32::MAX);
            let pct = (f64::from(hits) / denom * 1000.0).round() / 10.0;
            SkillCoverage {
                version: latest_versions.get(&skill_name).cloned(),
                skill_name,
                member_coverage_pct: pct,
                member_hits: hits,
            }
        })
        .collect();
    skills.sort_by(|a, b| {
        b.member_coverage_pct
            .partial_cmp(&a.member_coverage_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.skill_name.cmp(&b.skill_name))
    });

    Ok(Some(CohortDetail {
        summary: CohortSummary {
            cohort_id: cohort_id.to_string(),
            vertical,
            trust_grade_band: band_label.to_string(),
            trust_grade_min: lo,
            trust_grade_max: hi,
            member_count,
            skill_count,
        },
        members,
        skills,
    }))
}

fn parse_cohort_id(raw: &str) -> Option<(String, String, u8, u8)> {
    let (vertical, band_label) = raw.split_once("::")?;
    if vertical.is_empty() || band_label.is_empty() {
        return None;
    }
    let &(lo, hi, _) = BANDS.iter().find(|(_, _, label)| *label == band_label)?;
    Some((vertical.to_string(), band_label.to_string(), lo, hi))
}

fn worker_ids(workers: &[(String, String, u8)]) -> Vec<String> {
    workers.iter().map(|(id, _, _)| id.clone()).collect()
}

async fn fetch_workers(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<(String, String, u8)>> {
    let rows = sqlx::query(
        "SELECT id, vertical_id, trust_grade FROM worker WHERE tenant_id = ?",
    )
    .bind(tenant_id.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("cohort: list workers: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.get("id");
        let vertical: String = row.get("vertical_id");
        let grade_raw: i64 = row.try_get("trust_grade").unwrap_or(0);
        let grade = u8::try_from(grade_raw.clamp(0, i64::from(u8::MAX))).unwrap_or(0);
        out.push((id, vertical, grade));
    }
    Ok(out)
}

async fn fetch_worker_skills(
    store: &SessionStore,
    worker_ids: &[String],
) -> ArgoResult<HashMap<String, Vec<String>>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    if worker_ids.is_empty() {
        return Ok(out);
    }
    let rows = sqlx::query("SELECT worker_id, skill_id FROM worker_skill")
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("cohort: list worker_skill: {e}")))?;
    let id_set: BTreeSet<&str> = worker_ids.iter().map(String::as_str).collect();
    for row in rows {
        let worker_id: String = row.get("worker_id");
        if !id_set.contains(worker_id.as_str()) {
            continue;
        }
        let skill_id: String = row.get("skill_id");
        out.entry(worker_id).or_default().push(skill_id);
    }
    Ok(out)
}

async fn latest_versions_for(
    store: &SessionStore,
    skill_names: &[String],
) -> ArgoResult<HashMap<String, String>> {
    let mut out: HashMap<String, String> = HashMap::new();
    if skill_names.is_empty() {
        return Ok(out);
    }
    let rows = sqlx::query("SELECT name, version FROM skill")
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("cohort: list skill versions: {e}")))?;
    let wanted: BTreeSet<&str> = skill_names.iter().map(String::as_str).collect();
    for row in rows {
        let name: String = row.get("name");
        if !wanted.contains(name.as_str()) {
            continue;
        }
        let version: String = row.get("version");
        let slot = out.entry(name).or_insert_with(|| version.clone());
        if version.as_str() > slot.as_str() {
            *slot = version;
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "cohort_tests.rs"]
mod tests;
