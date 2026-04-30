//! Per-table row count stats — used by `argo doctor` /
//! `argo db status` to summarise the workspace.
use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};

/// Per-table row counts.
#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct DbStats {
    /// `session` rows.
    pub sessions: i64,
    /// `message` rows.
    pub messages: i64,
    /// `tool_call` rows.
    pub tool_calls: i64,
    /// `audit_record` rows.
    pub audit_rows: i64,
    /// `routing_decision` rows.
    pub routing_decisions: i64,
    /// `judge_record` rows.
    pub judge_records: i64,
    /// `memory_entry` rows.
    pub memory_entries: i64,
    /// `skill` rows.
    pub skills: i64,
    /// `forge_run` rows.
    pub forge_runs: i64,
    /// `spend_ledger` rows.
    pub spend_rows: i64,
}

pub(super) async fn db_stats(pool: &Pool<Sqlite>) -> Result<DbStats, ArgoError> {
    let mut stats = DbStats::default();
    for (table, target) in [
        ("session", &mut stats.sessions),
        ("message", &mut stats.messages),
        ("tool_call", &mut stats.tool_calls),
        ("audit_record", &mut stats.audit_rows),
        ("routing_decision", &mut stats.routing_decisions),
        ("judge_record", &mut stats.judge_records),
        ("memory_entry", &mut stats.memory_entries),
        ("skill", &mut stats.skills),
        ("forge_run", &mut stats.forge_runs),
        ("spend_ledger", &mut stats.spend_rows),
    ] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("count {table}: {e}")))?;
        *target = n;
    }
    Ok(stats)
}
