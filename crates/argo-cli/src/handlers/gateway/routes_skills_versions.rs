//! `GET /api/v1/skills/<id-or-name>/versions` — the REAL version
//! timeline for a skill, with the eval-delta changelog joined from
//! `skill_reeval_history`.
//!
//! ## Real data only (NO THEATER — [`DEC-007`])
//!
//! Two existing sources, joined — no fabricated history:
//!
//! * **Version timeline** — [`argo_skill::list_versions_for_tenant`]
//!   reads the caller-tenant's `skill` table rows for the name,
//!   newest-first, surfacing the real `version`, `trust_grade` (from
//!   `provenance.source`), `superseded` flag (`superseded_by IS NOT
//!   NULL`), and `lifecycle_state`. Tenant-scoped on purpose: the
//!   non-scoped `list_versions` pooled every tenant that ever seeded the
//!   same skill name into one timeline (the demo "6× v0.1.0" symptom).
//! * **Eval-delta changelog** — each version is matched against its
//!   `skill_reeval_history` rows (the scheduled re-eval runs that also
//!   drive `/evals` and the telemetry card). For a version we surface
//!   the LATEST re-eval's `pass_rate`, its `previous_pass_rate`, the
//!   derived pass-rate delta in percentage points, the run count for
//!   that version, and the `regressed` decision. A version with no
//!   re-eval rows reports `eval: null` — the card shows an honest "no
//!   eval runs yet" line rather than inventing a promotion arc.
//!
//! There is no trust-grade *promotion-event* table, so this route does
//! NOT claim "v2 promoted from community to verified"; it reports the
//! version's current grade + its real eval-pass-rate movement, which is
//! the evidence we actually have.
//!
//! ## Tenant scoping
//!
//! The skill name is first resolved under the caller's tenant (active,
//! non-superseded). A caller can only list versions for a skill name
//! that exists in THEIR tenant; the `skill_reeval_history` join is by
//! `skill_name` (a cross-tenant catalog table, exactly as the telemetry
//! route documents), reachable only via a name the tenant owns.
//!
//! [`DEC-007`]: ../../../../../docs/skycot/decisions/DEC-007-no-theater-in-demo.md

use super::routes_skills_list_get::request_tenant_id;
use argo_skill::{list_versions_for_tenant, SkillVersion};
use argo_store::SessionStore;
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Idempotent mirror of the lazily-applied reeval-history schema (the
/// table is created on demand by `argo_skill::scheduled_reeval_cron`,
/// not the migration registry). Identical bootstrap to
/// [`super::routes_skills_telemetry`].
const REEVAL_SCHEMA_SQL: &str =
    include_str!("../../../../argo-store/migrations/121__skill_reeval_history.sql");

/// `GET /api/v1/skills/<id-or-name>/versions`. Accepts the catalog UUID
/// or the skill name (mirrors `/usage` + `/telemetry`). Returns
/// `{ skill_name, version_count, versions: [...] }`.
pub(super) async fn skill_versions_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    id_or_name: &str,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant_str = match request_tenant_id(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let skill_name = match resolve_skill_name(store, &tenant_str, id_or_name).await {
        Ok(name) => name,
        Err(resp) => return resp,
    };

    // Tenant-scoped: the version timeline shows ONLY this tenant's rows.
    // The non-scoped `list_versions` surfaced every tenant that ever
    // seeded the same skill name (the demo "6× v0.1.0, all ACTIVE"
    // symptom — boot-seeds under shifting workspace tenant_ids each
    // added a row, all leaking into one timeline).
    let versions = match list_versions_for_tenant(store, &skill_name, &tenant_str).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("list versions: {e}")),
    };

    if let Err(e) = ensure_reeval_schema(store).await {
        return server_error(&format!("reeval schema: {e}"));
    }
    let reevals = match fetch_reeval_rows(store, &skill_name).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("reeval history: {e}")),
    };

    let versions_json: Vec<Value> = versions
        .iter()
        .map(|v| version_json(v, &reevals))
        .collect();
    let payload = json!({
        "skill_name": skill_name,
        "version_count": versions.len(),
        "versions": versions_json,
    });
    ("200 OK", payload.to_string())
}

/// One `skill_reeval_history` row, decoded for the eval-delta join.
struct ReevalRow {
    skill_version: String,
    run_at: i64,
    pass_rate: Option<f64>,
    previous_pass_rate: Option<f64>,
    regressed: i64,
}

/// Project one [`SkillVersion`] plus its eval evidence into the card
/// envelope. The eval block is the LATEST re-eval for the version (by
/// `run_at`), or `null` when the version was never re-evaluated.
fn version_json(version: &SkillVersion, reevals: &[ReevalRow]) -> Value {
    let runs: Vec<&ReevalRow> = reevals
        .iter()
        .filter(|r| r.skill_version == version.version)
        .collect();
    let eval = latest_eval(&runs);
    json!({
        "id": version.id,
        "version": version.version,
        "trust_grade": version.trust_grade,
        "superseded": version.superseded,
        "lifecycle_state": version.lifecycle_state,
        "created_at": version.created_at_rfc3339,
        "eval": eval,
    })
}

/// Build the eval-delta object from a version's re-eval runs (the rows
/// are oldest→newest; the latest is the freshest signal). Returns
/// `Value::Null` when the version has no recorded runs.
fn latest_eval(runs: &[&ReevalRow]) -> Value {
    let Some(latest) = runs.last() else {
        return Value::Null;
    };
    let delta_pp = match (latest.pass_rate, latest.previous_pass_rate) {
        (Some(now), Some(prev)) => Some(((now - prev) * 100.0 * 10.0).round() / 10.0),
        _ => None,
    };
    json!({
        "pass_rate": latest.pass_rate,
        "previous_pass_rate": latest.previous_pass_rate,
        "delta_pp": delta_pp,
        "run_count": runs.len(),
        "decision": decision_for(latest),
        "ran_at": micros_to_rfc3339(latest.run_at),
    })
}

/// Derive the eval decision from the stored `regressed` flag + baseline
/// presence. Matches the telemetry + `/evals` snake_case enum exactly.
fn decision_for(row: &ReevalRow) -> &'static str {
    if row.regressed != 0 {
        "regressed"
    } else if row.previous_pass_rate.is_none() {
        "no_baseline"
    } else {
        "stable"
    }
}

/// Fetch every reeval row for `skill_name`, oldest → newest.
async fn fetch_reeval_rows(
    store: &Arc<SessionStore>,
    skill_name: &str,
) -> Result<Vec<ReevalRow>, argo_types::ArgoError> {
    let rows = sqlx::query(
        "SELECT skill_version, run_at, pass_rate, previous_pass_rate, regressed \
         FROM skill_reeval_history WHERE skill_name = ? ORDER BY run_at ASC",
    )
    .bind(skill_name)
    .fetch_all(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("skill_reeval_history select: {e}")))?;
    Ok(rows
        .iter()
        .map(|row| ReevalRow {
            skill_version: row.get("skill_version"),
            run_at: row.get("run_at"),
            pass_rate: row.get("pass_rate"),
            previous_pass_rate: row.get("previous_pass_rate"),
            regressed: row.get("regressed"),
        })
        .collect())
}

/// Resolve a skill's canonical NAME from either the catalog UUID or the
/// name, scoped to the caller's tenant + active rows. Mirrors
/// `routes_skills_telemetry::resolve_skill`, returning just the name
/// (the version-list join key).
async fn resolve_skill_name(
    store: &Arc<SessionStore>,
    tenant_str: &str,
    id_or_name: &str,
) -> Result<String, (&'static str, String)> {
    let row = if let Ok(uuid) = Uuid::parse_str(id_or_name) {
        let id_bytes = uuid.as_bytes().to_vec();
        sqlx::query(
            "SELECT s.name FROM skill s \
             WHERE s.id = ? AND s.tenant_id = ? \
               AND s.superseded_by IS NULL AND s.lifecycle_state = 'active' LIMIT 1",
        )
        .bind(&id_bytes)
        .bind(tenant_str)
        .fetch_optional(store.pool())
        .await
    } else {
        sqlx::query(
            "SELECT s.name FROM skill s \
             WHERE s.name = ? AND s.tenant_id = ? \
               AND s.superseded_by IS NULL AND s.lifecycle_state = 'active' \
             ORDER BY s.created_at DESC LIMIT 1",
        )
        .bind(id_or_name)
        .bind(tenant_str)
        .fetch_optional(store.pool())
        .await
    };
    match row {
        Ok(Some(r)) => Ok(r.try_get("name").unwrap_or_default()),
        Ok(None) => Err((
            "404 Not Found",
            json!({ "error": format!("no active skill '{id_or_name}' in this tenant") })
                .to_string(),
        )),
        Err(e) => Err((
            "500 Internal Server Error",
            json!({ "error": format!("skill lookup: {e}") }).to_string(),
        )),
    }
}

/// Apply the lazy `skill_reeval_history` schema (idempotent). Strips SQL
/// comments and runs each statement — identical to the telemetry route.
async fn ensure_reeval_schema(store: &Arc<SessionStore>) -> Result<(), argo_types::ArgoError> {
    let stripped = REEVAL_SCHEMA_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(store.pool())
            .await
            .map_err(|e| {
                argo_types::ArgoError::Provider(format!("ensure skill_reeval_history: {e}"))
            })?;
    }
    Ok(())
}

fn micros_to_rfc3339(micros: i64) -> String {
    Utc.timestamp_micros(micros)
        .single()
        .unwrap_or(chrono::DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

fn server_error(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": msg }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_skills_versions_tests.rs"]
mod tests;
