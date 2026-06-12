//! `GET /api/v1/skills/<id-or-name>/telemetry` — REAL learning-loop
//! telemetry for the skill drill-in's "Learning loop" card (Scene 4).
//!
//! This route RETIRES the hardcoded `web/lib/sample-skill-telemetry.ts`
//! fixture (the camera-visible "184 runs across 11 workers" MOCK). Every
//! field below comes from a real table; any field the fixture carried
//! that has NO real source is OMITTED rather than fabricated
//! (NO THEATER — [`DEC-007`]).
//!
//! Real data sources, per field:
//!
//! * `current_grade` — [`TrustGrade::from_source`] over the skill's
//!   `provenance.source`. This is the live trust grade the runtime gates
//!   on, identical to the `/usage` route and the `/skills` list.
//! * `run_count` — `COUNT(*)` of this skill's `skill_reeval_history`
//!   rows (the scheduled re-eval runs that drive `/evals`). A REAL
//!   count, not a constant.
//! * `distinct_workers` — [`IdentityStore::count_workers_with_skill_global`]
//!   (the same cross-tenant adoption count the `/usage` route surfaces;
//!   aggregate integer only, no per-tenant identity leaks).
//! * `eval_history` — every `skill_reeval_history` row for the skill,
//!   oldest → newest: `version`, `pass_rate`, `previous_pass_rate`,
//!   `decision` (stable / regressed / no_baseline — same derivation the
//!   `/evals` route uses), and `ran_at`. These are REAL eval re-runs.
//!   NB: this is the skill's eval-trend history, NOT a fabricated
//!   trust-grade promotion arc — there is no promotion-event table, so
//!   we do not invent one.
//! * `adoption_trend` — cumulative distinct-worker adoption over the
//!   last 30 days, bucketed from `worker_skill.created_at` across all
//!   tenants (the same `learned_at` grain the `learned-this-month`
//!   route buckets). A REAL `total workers who had learned this skill by
//!   day D` series, oldest → newest.
//! * `recent_runs` — the newest 3 `skill_reeval_history` rows:
//!   `version`, `pass_rate`, `decision`, `ran_at`. The fixture's
//!   `worker_name` / `worker_id` / `latency_ms` are OMITTED — the reeval
//!   table carries no worker attribution and no latency, so inventing
//!   them would be theater.
//!
//! Empty state: a skill with zero reeval rows and zero adoption returns
//! a real, honest empty envelope (`run_count: 0`, empty arrays) — the
//! card renders its "earns A through real workers" empty state, never
//! the fixture.
//!
//! Tenant scoping: the skill row lookup is tenant-scoped (active,
//! non-superseded rows under the caller's tenant). `skill_reeval_history`
//! itself is a cross-tenant catalog table (re-eval runs target the
//! global catalog by `skill_id`; see its migration), so — exactly like
//! the `/evals` route — the scope is enforced on the skill-resolution
//! side: a caller can only reach reeval rows for a `skill_id` that
//! resolves under THEIR tenant. `adoption_trend` / `distinct_workers`
//! are deliberately cross-tenant aggregates (counts only).
//!
//! [`DEC-007`]: ../../../../../docs/skycot/decisions/DEC-007-no-theater-in-demo.md

use super::routes_skills_list_get::request_tenant_id;
use argo_identity::IdentityStore;
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Idempotent mirror of the lazily-applied reeval-history schema. The
/// table is NOT in the migration registry (it is created on demand by
/// `argo_skill::scheduled_reeval_cron`), so a store that has never run a
/// reeval tick would hit "no such table". We apply the same
/// `CREATE TABLE IF NOT EXISTS` defensively before the read — identical
/// to `routes_evals::ensure_reeval_schema`.
const REEVAL_SCHEMA_SQL: &str =
    include_str!("../../../../argo-store/migrations/121__skill_reeval_history.sql");

/// Number of trailing days the `adoption_trend` series covers.
const ADOPTION_TREND_DAYS: i64 = 30;

/// `GET /api/v1/skills/<id-or-name>/telemetry`. Accepts either the
/// catalog UUID (the SPA's skill-detail page keys on it) or the skill
/// name (marketplace card paths), mirroring the `/usage` route's
/// name→id tolerance.
pub(super) async fn skill_telemetry_response(
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
    let (uuid, skill_name, source) =
        match resolve_skill(store, &tenant_str, id_or_name).await {
            Ok(v) => v,
            Err(resp) => return resp,
        };
    if let Err(e) = ensure_reeval_schema(store).await {
        return server_error(&format!("reeval schema: {e}"));
    }
    let id_string = uuid.to_string();

    let reevals = match fetch_reeval_rows(store, &id_string).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("reeval history: {e}")),
    };

    let identity = IdentityStore::new(store);
    let distinct_workers = match identity.count_workers_with_skill_global(&id_string).await {
        Ok(c) => c,
        Err(e) => return server_error(&format!("adoption count: {e}")),
    };
    let learned_at = match identity.skill_learned_at_timestamps(&id_string).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("adoption trend: {e}")),
    };

    let now = Utc::now();
    let payload = json!({
        "skill_id": id_string,
        "skill_name": skill_name,
        "current_grade": TrustGrade::from_source(&source),
        "run_count": reevals.len(),
        "distinct_workers": distinct_workers,
        "eval_history": eval_history_json(&reevals),
        "adoption_trend": adoption_trend(&learned_at, now, ADOPTION_TREND_DAYS),
        "recent_runs": recent_runs_json(&reevals, 3),
    });
    ("200 OK", payload.to_string())
}

/// One `skill_reeval_history` row, decoded for the telemetry projection.
struct ReevalRow {
    skill_version: String,
    run_at: i64,
    pass_rate: Option<f64>,
    previous_pass_rate: Option<f64>,
    regressed: i64,
}

/// Fetch every reeval row for `skill_id`, oldest → newest (so the trend
/// reads left-to-right). Empty when the skill has never been re-evaluated.
async fn fetch_reeval_rows(
    store: &Arc<SessionStore>,
    skill_id: &str,
) -> Result<Vec<ReevalRow>, argo_types::ArgoError> {
    let rows = sqlx::query(
        "SELECT skill_version, run_at, pass_rate, previous_pass_rate, regressed \
         FROM skill_reeval_history WHERE skill_id = ? ORDER BY run_at ASC",
    )
    .bind(skill_id)
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

/// Project the reeval rows into the `eval_history` array (oldest →
/// newest). Each entry is a REAL eval re-run.
fn eval_history_json(rows: &[ReevalRow]) -> Vec<Value> {
    rows.iter()
        .map(|r| {
            json!({
                "version": r.skill_version,
                "pass_rate": r.pass_rate,
                "previous_pass_rate": r.previous_pass_rate,
                "decision": decision_for(r),
                "ran_at": micros_to_rfc3339(r.run_at),
            })
        })
        .collect()
}

/// The newest `n` reeval rows, newest-first, for the "recent runs"
/// block. Reuses the oldest→newest vec by taking the tail and reversing.
fn recent_runs_json(rows: &[ReevalRow], n: usize) -> Vec<Value> {
    let start = rows.len().saturating_sub(n);
    rows[start..]
        .iter()
        .rev()
        .map(|r| {
            json!({
                "version": r.skill_version,
                "pass_rate": r.pass_rate,
                "decision": decision_for(r),
                "ran_at": micros_to_rfc3339(r.run_at),
            })
        })
        .collect()
}

/// Derive the eval decision from the stored `regressed` flag + baseline
/// presence. Values match the `/evals` route's `snake_case` decision
/// enum exactly (`stable` / `regressed` / `no_baseline`).
fn decision_for(row: &ReevalRow) -> &'static str {
    if row.regressed != 0 {
        "regressed"
    } else if row.previous_pass_rate.is_none() {
        "no_baseline"
    } else {
        "stable"
    }
}

/// Build the cumulative adoption series: for each of the last
/// `days` calendar days (oldest → newest), the number of distinct
/// workers (across all tenants) who had learned the skill by the END of
/// that day. `learned_at` strings are `worker_skill.created_at` values
/// (`datetime('now')` → `YYYY-MM-DD HH:MM:SS` UTC). Rows that fail to
/// parse are dropped (a malformed timestamp never inflates the count).
fn adoption_trend(learned_at: &[String], now: chrono::DateTime<Utc>, days: i64) -> Vec<i64> {
    // Parse each learned-at into an epoch-second; drop unparseable rows.
    let learned_secs: Vec<i64> = learned_at
        .iter()
        .filter_map(|s| parse_worker_skill_ts(s))
        .collect();
    let end_of_today = now
        .date_naive()
        .and_hms_opt(23, 59, 59)
        .map(|d| d.and_utc().timestamp())
        .unwrap_or_else(|| now.timestamp());
    let day_secs = 86_400_i64;
    (0..days)
        .map(|i| {
            // Cutoff = end of the day `(days-1-i)` days before today.
            let cutoff = end_of_today - (days - 1 - i) * day_secs;
            learned_secs.iter().filter(|&&t| t <= cutoff).count() as i64
        })
        .collect()
}

/// Parse a `worker_skill.created_at` string. SQLite's `datetime('now')`
/// emits `YYYY-MM-DD HH:MM:SS` (UTC, no offset). Returns the epoch
/// second, or `None` if the string is in neither that form nor RFC3339.
fn parse_worker_skill_ts(raw: &str) -> Option<i64> {
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        return Some(naive.and_utc().timestamp());
    }
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp())
}

/// Convert a `timestamp_micros` integer (the `run_at` unit) to an
/// RFC3339 UTC string. Out-of-range values fall back to the Unix epoch
/// rather than panicking.
fn micros_to_rfc3339(micros: i64) -> String {
    Utc.timestamp_micros(micros)
        .single()
        .unwrap_or(chrono::DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

/// Resolve a skill's `(uuid, name, source)` accepting either the catalog
/// UUID or the skill name, scoped to the caller's tenant + active rows.
/// Mirrors `routes_skills_usage::resolve_skill_for_usage`.
async fn resolve_skill(
    store: &Arc<SessionStore>,
    tenant_str: &str,
    id_or_name: &str,
) -> Result<(Uuid, String, String), (&'static str, String)> {
    let row = if let Ok(uuid) = Uuid::parse_str(id_or_name) {
        let id_bytes = uuid.as_bytes().to_vec();
        sqlx::query(
            "SELECT s.id, s.name, s.source FROM skill s \
             WHERE s.id = ? AND s.tenant_id = ? \
               AND s.superseded_by IS NULL AND s.lifecycle_state = 'active' LIMIT 1",
        )
        .bind(&id_bytes)
        .bind(tenant_str)
        .fetch_optional(store.pool())
        .await
    } else {
        sqlx::query(
            "SELECT s.id, s.name, s.source FROM skill s \
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
        Ok(Some(r)) => {
            let id_blob: Vec<u8> = r.try_get("id").unwrap_or_default();
            let Some(uuid) = uuid_from_blob(&id_blob) else {
                return Err((
                    "500 Internal Server Error",
                    json!({ "error": { "code": "internal_error", "message": "skill id column not a 16-byte uuid" } }).to_string(),
                ));
            };
            Ok((
                uuid,
                r.try_get("name").unwrap_or_default(),
                r.try_get("source").unwrap_or_default(),
            ))
        }
        Ok(None) => Err((
            "404 Not Found",
            json!({ "error": { "code": "not_found", "message": format!("no active skill '{id_or_name}' in this tenant") } })
                .to_string(),
        )),
        Err(e) => Err((
            "500 Internal Server Error",
            json!({ "error": { "code": "internal_error", "message": format!("skill lookup: {e}") } }).to_string(),
        )),
    }
}

/// Decode a 16-byte BLOB id column into a `Uuid`. Returns `None` for any
/// other length so a malformed row surfaces a 500 rather than panicking.
fn uuid_from_blob(blob: &[u8]) -> Option<Uuid> {
    if blob.len() == 16 {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(blob);
        Some(Uuid::from_bytes(arr))
    } else {
        None
    }
}

/// Apply the lazy `skill_reeval_history` schema (idempotent). Strips SQL
/// comments and runs each statement, matching the bootstrap the gateway
/// eval route + the reeval cron use.
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

fn server_error(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal_error", "message": msg } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_skills_telemetry_tests.rs"]
mod tests;
