//! `argo worker charter` subcommands.
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum WorkerCharterCmd {
    /// Set or replace the worker's charter and/or KPIs.
    Set {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Path to a markdown file holding the charter body.
        #[arg(long)]
        file: PathBuf,
        /// Optional path to a JSON file holding the KPIs object.
        #[arg(long)]
        kpis: Option<PathBuf>,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Print the worker's charter and/or KPIs.
    Get {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Render the charter markdown to stdout (default).
        #[arg(long)]
        md: bool,
        /// Emit `{charter, kpis}` JSON.
        #[arg(long)]
        json: bool,
    },
    /// Null out the charter and KPIs for one worker.
    Clear {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
}
