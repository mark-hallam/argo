//! `GET /api/v1/workers/<id>/evals` — per-worker eval dashboard.
//!
//! A worker's "evals" are the scheduled re-evaluation history of the
//! **skills that worker has learned**. There is no per-worker eval-run
//! table; the data grain is per-skill (`skill_reeval_history`, a global
//! marketplace table keyed by `skill_id`). This handler resolves the
//! worker's learned skill ids (the tenant-scoped `worker_skill` overlay
//! via [`IdentityStore::skills_for_worker`]) and surfaces the reeval
//! rows for exactly those skills as the worker's eval runs.
//!
//! Tenant scoping: `skill_reeval_history` itself carries no `tenant_id`
//! (skills are a cross-tenant catalog), so the scope is enforced on the
//! worker side — [`helpers::resolve_worker_ctx`] looks the worker up
//! under the workspace tenant (cross-tenant / unknown both surface as
//! 404), and we only ever query reeval rows for skill ids that *this*
//! tenant's worker holds. A worker in another tenant never reaches the
//! reeval query.
//!
//! JSON output shape is held identical to the prior mock so the SPA
//! contract (`web/lib/api/evals.ts` — `EvalDashboardPayload` =
//! `{ workerId, latest, history }` of `EvalRun`s) is unchanged, with one
//! honest addition: `latest` is `null` (not a fabricated run) when the
//! worker has no learned skills or none of them has reeval history.
//!
//! Sensible deviations from a "real per-worker eval run" ideal:
//! - `fixtureCount` / `meanScore` / `durationMs` / `fixtures[]` are not
//!   stored per reeval row (the table keeps only the aggregate
//!   `pass_rate`). They are surfaced as best-effort derived values
//!   (`fixtures` empty, `meanScore` = `passRate`, counters `0`) rather
//!   than invented detail — the camera-visible signal (pass rate,
//!   version, regression decision, trend) is all real.
//! - `decision` is derived from the stored `regressed` flag and the
//!   presence of a `previous_pass_rate` baseline, matching the
//!   `snake_case` `EvalRegressionDecision` the SPA's `EvalDecision`
//!   type expects (`stable` / `regressed` / `no_baseline`).

use super::routes_worker_skills_helpers as helpers;
use argo_audit::AuditKind;
use argo_identity::IdentityStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

/// Idempotent mirror of `crates/argo-store/migrations/121__skill_reeval_history.sql`.
/// The table is lazily applied by `argo_skill::scheduled_reeval_cron`
/// (it is NOT in the migration registry), so a store that has never run
/// a reeval tick would otherwise hit "no such table" here. We apply the
/// same `CREATE TABLE IF NOT EXISTS` defensively before the read.
const REEVAL_SCHEMA_SQL: &str =
    include_str!("../../../../argo-store/migrations/121__skill_reeval_history.sql");

/// One `skill_reeval_history` row, decoded for projection into an
/// `EvalRun`. Mirrors the column order of [`HISTORY_SELECT`].
struct ReevalRow {
    eval_run_id: String,
    skill_id: String,
    skill_name: String,
    skill_version: String,
    run_at: i64,
    pass_rate: Option<f64>,
    previous_pass_rate: Option<f64>,
    regressed: i64,
}

const HISTORY_SELECT: &str = "SELECT eval_run_id, skill_id, skill_name, skill_version, \
     run_at, pass_rate, previous_pass_rate, regressed \
     FROM skill_reeval_history WHERE skill_id IN ";

/// `GET /api/v1/workers/<id>/evals` entry point.
pub(super) async fn list_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match helpers::parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ctx = match helpers::resolve_worker_ctx(store, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };

    if let Err(e) = ensure_reeval_schema(store).await {
        return helpers::server_error(&format!("reeval schema: {e}"));
    }

    // The worker's learned skill ids (TEXT UUIDs). `worker_skill.skill_id`
    // and `skill_reeval_history.skill_id` are both the hyphenated UUID
    // string, so they join directly (unlike `skill.id`, stored as BLOB).
    let skill_ids: Vec<String> = match IdentityStore::new(store)
        .skills_for_worker(worker_id)
        .await
    {
        Ok(rows) => rows.into_iter().map(|(id, _prof)| id).collect(),
        Err(e) => return helpers::server_error(&format!("skills_for_worker: {e}")),
    };

    let runs = match fetch_runs_for_skills(store, &worker_id.to_string(), &skill_ids).await {
        Ok(v) => v,
        Err(e) => return helpers::server_error(&format!("reeval history: {e}")),
    };

    let latest = runs.first().cloned().unwrap_or(Value::Null);
    let history = runs.clone();
    let body = json!({
        "workerId": worker_id.to_string(),
        "latest": latest,
        "history": history,
    });
    helpers::emit_audit(
        store,
        req,
        &ctx,
        AuditKind::HttpRequest,
        "worker.evals.viewed",
        json!({
            "skill_count": skill_ids.len(),
            "run_count": runs.len(),
        }),
    )
    .await;
    ("200 OK", body.to_string())
}

/// Apply the lazy `skill_reeval_history` schema (idempotent). Strips
/// SQL comments and runs each statement, matching the bootstrap the
/// `argo-skill` reeval cron uses.
async fn ensure_reeval_schema(store: &Arc<SessionStore>) -> ArgoResult<()> {
    let stripped = REEVAL_SCHEMA_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("ensure skill_reeval_history: {e}")))?;
    }
    Ok(())
}

/// Fetch every reeval row for `skill_ids`, newest-first, projected into
/// the SPA `EvalRun` shape. Empty `skill_ids` short-circuits to an empty
/// vec (no SQL) so a skill-less worker is a real empty, not a 500.
async fn fetch_runs_for_skills(
    store: &Arc<SessionStore>,
    worker_id: &str,
    skill_ids: &[String],
) -> ArgoResult<Vec<Value>> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("{HISTORY_SELECT}({placeholders}) ORDER BY run_at DESC");
    let mut query = sqlx::query(&sql);
    for id in skill_ids {
        query = query.bind(id);
    }
    let rows = query
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_reeval_history select: {e}")))?;
    Ok(rows.iter().map(|row| run_json(worker_id, &decode_row(row))).collect())
}

/// Decode one SQLite row into a [`ReevalRow`]. `sqlx::Row::get` panics
/// on a type/column mismatch, but the columns are pinned by
/// [`HISTORY_SELECT`] against the fixed schema, so this is total for
/// any row the query returns.
fn decode_row(row: &sqlx::sqlite::SqliteRow) -> ReevalRow {
    ReevalRow {
        eval_run_id: row.get("eval_run_id"),
        skill_id: row.get("skill_id"),
        skill_name: row.get("skill_name"),
        skill_version: row.get("skill_version"),
        run_at: row.get("run_at"),
        pass_rate: row.get("pass_rate"),
        previous_pass_rate: row.get("previous_pass_rate"),
        regressed: row.get("regressed"),
    }
}

/// Project one [`ReevalRow`] into the SPA's `EvalRun` JSON shape. Fields
/// the table does not carry are filled with honest derived values (see
/// the module docs): `meanScore` = `passRate`, empty `fixtures`, zero
/// `fixtureCount` / `durationMs`.
fn run_json(worker_id: &str, row: &ReevalRow) -> Value {
    let pass_rate = row.pass_rate.unwrap_or(0.0);
    json!({
        "id": row.eval_run_id,
        "workerId": worker_id,
        "skill": skill_label(row),
        "version": row.skill_version,
        "createdAt": micros_to_rfc3339(row.run_at),
        "status": status_for(row.pass_rate),
        "passRate": pass_rate,
        "meanScore": pass_rate,
        "fixtureCount": 0,
        "durationMs": 0,
        "decision": decision_for(row),
        "fixtures": Vec::<Value>::new(),
        "skillId": row.skill_id,
        "previousPassRate": row.previous_pass_rate,
    })
}

/// Prefer the stored `skill_name`; fall back to the id when a row
/// pre-dates the name capture.
fn skill_label(row: &ReevalRow) -> &str {
    if row.skill_name.is_empty() {
        &row.skill_id
    } else {
        &row.skill_name
    }
}

/// `EvalRunStatus` for a pass rate: a run with no recorded rate (the
/// eval itself errored) is `running` (in-flight / indeterminate); ≥0.8
/// is `passed`, else `failed`. Matches the SPA's `EvalRunStatus`.
fn status_for(pass_rate: Option<f64>) -> &'static str {
    match pass_rate {
        None => "running",
        Some(r) if r >= 0.8 => "passed",
        Some(_) => "failed",
    }
}

/// Derive the `EvalDecision` from the stored `regressed` flag and
/// whether a baseline existed. Values match the SPA's `snake_case`
/// `EvalDecision` enum exactly.
fn decision_for(row: &ReevalRow) -> &'static str {
    if row.regressed != 0 {
        "regressed"
    } else if row.previous_pass_rate.is_none() {
        "no_baseline"
    } else {
        "stable"
    }
}

/// Convert a `timestamp_micros` integer (the unit `run_at` is stored in)
/// to an RFC3339 UTC string for the SPA's `createdAt`. Out-of-range
/// values fall back to the Unix epoch rather than panicking.
fn micros_to_rfc3339(micros: i64) -> String {
    Utc.timestamp_micros(micros)
        .single()
        .unwrap_or(chrono::DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

#[cfg(test)]
#[path = "routes_evals_tests.rs"]
mod tests;
