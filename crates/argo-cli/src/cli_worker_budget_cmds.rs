//! `argo worker budget` sub-enum (Stream `claude-worker-budget-cli`).
//!
//! Lives as a sibling of `cli_worker_cmds.rs` so the parent stays under
//! the 500-LOC project ceiling, mirroring the channels / archive / charter
//! sub-enum pattern.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerBudgetCmd {
    /// Set or update the per-worker spend caps. At least one of
    /// `--daily`, `--monthly`, or `--clear` is required. Caps are
    /// stored as USD micros on the worker's auto-approval policy
    /// JSON; the dispatch gate already honours `--daily` (the
    /// monthly cap is recorded for `argo worker budget show` /
    /// future gate work — see PR notes).
    Set {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Daily ceiling in USD (decimal dollars; e.g. `5`, `5.50`,
        /// `0` to deny all). Stored as integer micros.
        #[arg(long)]
        daily: Option<String>,
        /// Monthly ceiling in USD (decimal dollars).
        #[arg(long)]
        monthly: Option<String>,
        /// Drop both cap fields and fall back to the tenant default.
        #[arg(long)]
        clear: bool,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Print the worker's current caps + period-to-date spend.
    Show {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
}
