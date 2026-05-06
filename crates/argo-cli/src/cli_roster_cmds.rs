//! `argo roster` subcommands (V1.3 — `claude-worker-roster`).
//! Sibling-file pattern; cli.rs keeps only the single Roster
//! variant pointing at the RosterCmd enum here.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum RosterCmd {
    /// Create a new roster (group of workers + shared policy).
    Create {
        /// Tenant id this roster belongs to.
        #[arg(long)]
        tenant: String,
        /// Operator-friendly group name (unique per tenant).
        name: String,
        /// Path to a JSON file describing the AutoApprovalPolicy.
        /// Falls back to the disabled default when omitted.
        #[arg(long)]
        policy: Option<std::path::PathBuf>,
        /// Path to a JSON file describing the NotifyChannelRef
        /// list. Falls back to an empty list when omitted.
        #[arg(long)]
        channels: Option<std::path::PathBuf>,
        /// Optional group-level spend ceiling in whole USD.
        #[arg(long)]
        budget_cap_usd: Option<f64>,
        /// Emit JSON instead of the human-friendly recap.
        #[arg(long)]
        json: bool,
    },
    /// List rosters in a tenant.
    List {
        /// Tenant id to filter by.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of the table view.
        #[arg(long)]
        json: bool,
    },
    /// Show a roster (with members hydrated).
    Show {
        /// Tenant id this roster belongs to.
        #[arg(long)]
        tenant: String,
        /// Roster name OR uuid.
        roster: String,
        /// Emit JSON instead of the multi-line view.
        #[arg(long)]
        json: bool,
    },
    /// Add a worker to a roster.
    AddMember {
        /// Tenant id this roster belongs to.
        #[arg(long)]
        tenant: String,
        /// Roster name OR uuid.
        roster: String,
        /// Worker uuid to add.
        worker: String,
        /// Emit JSON instead of the human-friendly confirmation.
        #[arg(long)]
        json: bool,
    },
    /// Remove a worker from a roster.
    RemoveMember {
        /// Tenant id this roster belongs to.
        #[arg(long)]
        tenant: String,
        /// Roster name OR uuid.
        roster: String,
        /// Worker uuid to remove.
        worker: String,
        /// Emit JSON instead of the human-friendly confirmation.
        #[arg(long)]
        json: bool,
    },
}
