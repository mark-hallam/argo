//! Schema migration v49 — per-Forge-run cost attribution columns
//! on `forge_run` for Stream `mission-forge-cost-attribution`
//! (MISSION_BACKLOG Section 6 #206).
//!
//! Today `forge_run.spend_usd_micros` records aggregate LLM spend
//! only — operators tuning Forge budget can't see per-skill or
//! per-vertical cost breakdowns. This migration breaks that single
//! counter out into 7 structured columns: per-call token counts
//! (input / output / cache_read), model-priced LLM cost, E2B
//! sandbox time + cost, and a stamped `total_cost_usd_micros`.
//!
//! Slot rationale: v48 (`persona_revision`, brief
//! `mission-persona-evolution-audit-revert` PR #1299) occupied the
//! prior versioned slot when this brief landed; v49 is the next
//! free versioned slot. The brief's "implementer verifies slot
//! ≥v47 to avoid v50/51 collision with `mission-escalation-on-no-
//! reply`" directive points at v49 since v47/v48 are taken on
//! origin/main and v50/v51 are reserved for the escalation brief.
//! Reconciles raw SQL `53__forge_run_cost.sql` (filename slots
//! 49..52 are taken by `worker_archived_at`,
//! `worker_trust_grade`, `audit_worker_deleted_at` +
//! `worker_skill`, and `persona_revision` respectively).
//!
//! UP adds the 7 nullable-defaulting columns plus two query
//! indexes used by the `argo forge cost-report` CLI:
//! `(total_cost_usd_micros DESC)` for the "most-expensive runs"
//! sort and `(skill_name, started_at DESC)` for the
//! per-skill / since-window aggregation. Idempotent via
//! `PRAGMA table_info(forge_run)` column probes — re-running the
//! migration over a partially-applied schema is safe.
//!
//! DOWN drops the indexes and the columns in reverse order.
//! SQLite ≥ 3.35 supports `ALTER TABLE … DROP COLUMN`; all 7
//! columns are nullable-defaulting INTEGERs with no FK references
//! so dropping them is safe.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for (col, ddl) in COST_COLUMNS {
        if !column_exists(tx, "forge_run", col).await? {
            exec(tx, "v49 up", ddl).await?;
        }
    }
    exec(
        tx,
        "v49 up",
        "CREATE INDEX IF NOT EXISTS idx_forge_run_total_cost \
         ON forge_run(total_cost_usd_micros DESC)",
    )
    .await?;
    exec(
        tx,
        "v49 up",
        "CREATE INDEX IF NOT EXISTS idx_forge_run_skill_started_at \
         ON forge_run(skill_name, started_at DESC)",
    )
    .await?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    exec(
        tx,
        "v49 down",
        "DROP INDEX IF EXISTS idx_forge_run_skill_started_at",
    )
    .await?;
    exec(
        tx,
        "v49 down",
        "DROP INDEX IF EXISTS idx_forge_run_total_cost",
    )
    .await?;
    for (col, _) in COST_COLUMNS.iter().rev() {
        if column_exists(tx, "forge_run", col).await? {
            let sql = format!("ALTER TABLE forge_run DROP COLUMN {col}");
            exec(tx, "v49 down", &sql).await?;
        }
    }
    Ok(())
}

const COST_COLUMNS: &[(&str, &str)] = &[
    (
        "llm_input_tokens",
        "ALTER TABLE forge_run ADD COLUMN llm_input_tokens INTEGER NOT NULL DEFAULT 0",
    ),
    (
        "llm_output_tokens",
        "ALTER TABLE forge_run ADD COLUMN llm_output_tokens INTEGER NOT NULL DEFAULT 0",
    ),
    (
        "llm_cache_read_tokens",
        "ALTER TABLE forge_run ADD COLUMN llm_cache_read_tokens INTEGER NOT NULL DEFAULT 0",
    ),
    (
        "llm_cost_usd_micros",
        "ALTER TABLE forge_run ADD COLUMN llm_cost_usd_micros INTEGER NOT NULL DEFAULT 0",
    ),
    (
        "e2b_seconds",
        "ALTER TABLE forge_run ADD COLUMN e2b_seconds INTEGER NOT NULL DEFAULT 0",
    ),
    (
        "e2b_cost_usd_micros",
        "ALTER TABLE forge_run ADD COLUMN e2b_cost_usd_micros INTEGER NOT NULL DEFAULT 0",
    ),
    (
        "total_cost_usd_micros",
        "ALTER TABLE forge_run ADD COLUMN total_cost_usd_micros INTEGER NOT NULL DEFAULT 0",
    ),
];

async fn exec<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    ctx: &str,
    sql: &str,
) -> Result<(), ArgoError> {
    sqlx::query(sql)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("{ctx}: {e}")))?;
    Ok(())
}

async fn column_exists<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    table: &str,
    column: &str,
) -> Result<bool, ArgoError> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v49 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}
