//! Workflow run-history seeder for the canonical v1 demo.
//!
//! Scene 5 of the v1 demo runbook lands on the
//! `customer_research_followup` workflow detail page, whose "Run
//! history" panel (`routes_workflow_runs.rs` →
//! `GET /api/v1/verticals/<v>/workflows/<n>/runs`) reads the
//! migration-18 `workflow_run` table. Before this seeder the demo
//! tenant had zero `workflow_run` rows for that workflow, so the panel
//! rendered an empty run count / "—" success rate while the workflow
//! definition itself was seeded and live — a NO-THEATER gap (real
//! endpoint, real workflow, empty data).
//!
//! This module inserts **real** `workflow_run` rows whose
//! `workflow_id` matches the compound key the live detail route builds
//! for the seeded workflow, so the aggregate stats (run count, success
//! rate, avg + p95 latency, last-run-at) and the recent-runs table
//! populate from genuine rows.
//!
//! ## `workflow_id` matching (load-bearing)
//!
//! `workflow_run` is **not** tenant-scoped (migration 18's SAFETY
//! note): the run-history endpoint matches `workflow_id IN
//! (compound_key, bare_key)` where `compound_key =
//! "{vertical_id}/{name}"` and `bare_key = "{name}"`. In demo-live the
//! `/workflows` list comes from the real `workflow_definition` table
//! via `GET /api/v1/workflows`, which surfaces this workflow under its
//! seeded vertical `marketplace` (`workflow::DEMO_VERTICAL_ID`), so the
//! detail route the operator opens is
//! `/workflows/marketplace/customer_research_followup` and the runs
//! request builds `compound_key =
//! "marketplace/customer_research_followup"`. We seed rows under that
//! exact compound key so the live panel matches them.
//!
//! Raw SQL (not a store helper) matches the seed-crate convention; the
//! column set mirrors `crates/argo-store/migrations/18__workflow_runs.sql`
//! exactly. `workflow_yaml` carries the same TOML body the workflow
//! definition seeded so a row drilled into is internally consistent.
//!
//! Idempotent: if any `workflow_run` row already exists for the
//! workflow's compound key the whole block is skipped, so re-seeding
//! never duplicates the run history.

use argo_store::SessionStore;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::SeedResult;

/// Compound `workflow_id` key the live detail route builds for the
/// seeded workflow (`{vertical}/{name}`). Mirrors
/// `workflow::DEMO_VERTICAL_ID` + `workflow::DEMO_WORKFLOW_NAME`; kept
/// as a literal here so the seed-crate convention (each `seed_*` module
/// is self-contained raw SQL) holds and a refactor of `workflow.rs`
/// can't silently move the runs out from under the panel without this
/// test-covered constant also changing.
const WORKFLOW_RUN_ID: &str = "marketplace/customer_research_followup";

/// The workflow body stamped into each run's `workflow_yaml` column so
/// a run row is internally consistent with the seeded definition.
const WORKFLOW_YAML: &str = "schema_version = \"1\"\n\n[workflow]\nname = \"customer_research_followup\"\n";

/// One canonical demo run. `minutes_ago` is the run start offset from
/// now; `duration_secs` is the wall time; `ok` = `Some(true)` success,
/// `Some(false)` failed, `None` still running (no `finished_at`).
struct DemoRun {
    minutes_ago: i64,
    duration_secs: i64,
    ok: Option<bool>,
}

/// Seven runs spread across the past ~12 days: six finished (five
/// succeeded, one failed) plus one still running. The mix gives the
/// Scene-5 panel a believable ~83% success rate, a non-trivial
/// avg/p95 latency, and at least one of each status chip
/// (success/failed/running) so the status filter has something to
/// show.
const DEMO_RUNS: &[DemoRun] = &[
    DemoRun {
        minutes_ago: 12, // still running
        duration_secs: 0,
        ok: None,
    },
    DemoRun {
        minutes_ago: 95,
        duration_secs: 7,
        ok: Some(true),
    },
    DemoRun {
        minutes_ago: 1_500, // ~yesterday
        duration_secs: 6,
        ok: Some(true),
    },
    DemoRun {
        minutes_ago: 2_900, // ~2 days
        duration_secs: 9,
        ok: Some(false), // the one failure
    },
    DemoRun {
        minutes_ago: 5_800, // ~4 days
        duration_secs: 5,
        ok: Some(true),
    },
    DemoRun {
        minutes_ago: 11_500, // ~8 days
        duration_secs: 8,
        ok: Some(true),
    },
    DemoRun {
        minutes_ago: 17_000, // ~12 days
        duration_secs: 6,
        ok: Some(true),
    },
];

/// Insert canonical `workflow_run` rows for the seeded
/// `customer_research_followup` workflow so the Scene-5 detail page's
/// run-history aggregates + recent-runs table render real data.
///
/// Returns the number of rows inserted (`7` on first run, `0` on
/// re-seed). Idempotent: a compound key that already has any
/// `workflow_run` row is skipped wholesale.
pub(crate) async fn seed_workflow_runs(store: &Arc<SessionStore>) -> SeedResult<usize> {
    if compound_key_has_runs(store).await? {
        return Ok(0);
    }
    let now = Utc::now().timestamp_micros();
    let mut inserted = 0usize;
    for run in DEMO_RUNS {
        insert_run(store, run, now).await?;
        inserted += 1;
    }
    Ok(inserted)
}

async fn compound_key_has_runs(store: &Arc<SessionStore>) -> SeedResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_run WHERE workflow_id = ?")
            .bind(WORKFLOW_RUN_ID)
            .fetch_one(store.pool())
            .await
            .map_err(|e| argo_types::ArgoError::Provider(format!("workflow_run probe: {e}")))?;
    Ok(count > 0)
}

async fn insert_run(store: &Arc<SessionStore>, run: &DemoRun, now: i64) -> SeedResult<()> {
    let started_at = now - run.minutes_ago * 60 * 1_000_000;
    // A running row (`ok = None`) has no finish time — the endpoint
    // treats `finished_at IS NULL` as "running"; a finished row's
    // finish = start + duration.
    let finished_at: Option<i64> = run.ok.map(|_| started_at + run.duration_secs * 1_000_000);
    let ok_int: Option<i64> = run.ok.map(i64::from);
    sqlx::query(
        "INSERT INTO workflow_run \
         (id, workflow_id, workflow_yaml, started_at, finished_at, ok, task_verdicts_json) \
         VALUES (?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(WORKFLOW_RUN_ID)
    .bind(WORKFLOW_YAML)
    .bind(started_at)
    .bind(finished_at)
    .bind(ok_int)
    .execute(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("insert workflow_run: {e}")))?;
    Ok(())
}

#[cfg(test)]
#[path = "seed_workflow_runs_tests.rs"]
mod tests;
