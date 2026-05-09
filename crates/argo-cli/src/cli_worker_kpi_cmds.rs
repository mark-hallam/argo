//! `argo worker kpi record/list/series` subcommand schema (Stream
//! `claude-worker-kpi-tracker`, mission claim 2 runtime evidence).
//! Lives in a sibling file so `cli_worker_cmds.rs` stays under the
//! 500-LOC per-file ceiling — same pattern used by
//! `cli_worker_archive_cmds.rs` and `cli_worker_charter_cmds.rs`.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerKpiCmd {
    /// Append one observation of `kpi` for `worker_id`.
    Record {
        /// Worker id (UUIDv7) the observation belongs to.
        worker_id: String,
        /// KPI name (e.g. `csat`, `reply_sla_hours`). Free-form;
        /// not validated against the worker's charter `kpi_json`
        /// definition list — that's a follow-up.
        kpi: String,
        /// Numeric value. Stored as SQLite REAL.
        value: f64,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Optional free-text note recorded alongside the value
        /// (e.g. "post-incident review", "customer migrated").
        #[arg(long)]
        note: Option<String>,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// List observed KPI values for a worker, newest first.
    List {
        /// Worker id (UUIDv7) to query.
        worker_id: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Filter to one KPI name. When omitted, all KPIs are returned.
        #[arg(long)]
        kpi: Option<String>,
        /// RFC3339 cutoff — only observations at or after this time
        /// are returned. Useful for "show me this week's csat" without
        /// counting rows.
        #[arg(long)]
        since: Option<String>,
        /// Emit JSON-lines (one row per observation) instead of a
        /// human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Roll observations into time buckets and report avg/min/max/count.
    Series {
        /// Worker id (UUIDv7) to aggregate.
        worker_id: String,
        /// KPI name to aggregate. Required — series is per-KPI.
        kpi: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Bucket granularity: `day`, `week` (Monday-stamped), or
        /// `month` (`YYYY-MM`).
        #[arg(long)]
        bucket: String,
        /// Emit JSON-lines (one row per bucket) instead of a
        /// human-readable table.
        #[arg(long)]
        json: bool,
    },
}
