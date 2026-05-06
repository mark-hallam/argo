//! `argo budget` subcommands for per-worker spend visibility.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum BudgetCmd {
    /// Show daily/monthly rollups, policy headroom, and last
    /// five charges for one worker.
    Show {
        worker_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List recent worker-budget charges.
    Recent {
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long)]
        worker: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        json: bool,
    },
}
