//! `argo worker costs` command schema. Kept out of
//! `cli_worker_cmds.rs` so the worker command enum has room for
//! future worker-family read-side commands.

use clap::{Args, ValueEnum};

#[derive(Args, Debug)]
pub struct WorkerCostsCmd {
    /// Worker id whose spend ledger should be queried.
    pub worker_id: String,
    /// Tenant id to scope the query. When omitted, all rows for
    /// the worker id are included.
    #[arg(long)]
    pub tenant: Option<String>,
    /// Only include charges at or after this RFC3339 timestamp.
    #[arg(long)]
    pub since: Option<String>,
    /// Group totals by UTC day or action id.
    #[arg(long, value_enum)]
    pub breakdown: Option<WorkerCostsBreakdown>,
    /// Number of recent charge rows to render in text/JSON.
    #[arg(long, default_value_t = 10)]
    pub limit: u32,
    /// Emit a stable JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum WorkerCostsBreakdown {
    /// Group charges by UTC calendar day.
    Day,
    /// Group charges by action id.
    Action,
}
