//! `argo handoff-rules` subcommands (V1.3 —
//! `claude-handoff-routing-rules`). Sibling-file pattern.
//!
//! Note: surfaced as the top-level `handoff-rules` command rather
//! than nested under `argo handoff` because the existing
//! `HandoffCmd` enum lives in `cli_session_cmds.rs` (outside this
//! brief's may-touch list). Functionally equivalent.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum HandoffRulesCmd {
    /// List rules for a tenant in priority order.
    List {
        /// Tenant id.
        #[arg(long)]
        tenant: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show one rule (uuid OR name lookup).
    Show {
        /// Tenant id.
        #[arg(long)]
        tenant: String,
        /// Rule uuid OR name.
        rule: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Add a rule from a JSON file describing the DSL.
    Add {
        /// Tenant id.
        #[arg(long)]
        tenant: String,
        /// Path to JSON describing the rule (name + when + then +
        /// priority + active).
        path: std::path::PathBuf,
        /// Emit JSON instead of the human-friendly recap.
        #[arg(long)]
        json: bool,
    },
    /// Remove a rule (uuid OR name).
    Remove {
        /// Tenant id.
        #[arg(long)]
        tenant: String,
        /// Rule uuid OR name.
        rule: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Simulate rule application against a hypothetical handoff
    /// without persisting anything.
    Test {
        /// Tenant id whose rules should be loaded.
        #[arg(long)]
        tenant: String,
        /// Hypothetical `expected_outcome` text.
        #[arg(long)]
        expected_outcome: String,
        /// Hypothetical hour-of-day UTC (`0..24`). Defaults to
        /// the current hour.
        #[arg(long)]
        hour: Option<u32>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}

/// `argo handoff-auctions` subcommands (Stream
/// `claude-skill-auction`). Surfaced as a top-level command for
/// the same reason as `handoff-rules` — `HandoffCmd` lives in
/// `cli_session_cmds.rs` outside the brief's may-touch list.
#[derive(Subcommand, Debug)]
pub enum HandoffAuctionsCmd {
    /// List recent auctions, newest-first. Optional `--status`
    /// filter (`open` / `closed` / `cancelled` / `no_bidders`).
    List {
        /// Filter by status string.
        #[arg(long)]
        status: Option<String>,
        /// Cap the number of rows printed. Default 100.
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show one auction by id with all its bids.
    Show {
        /// Auction id.
        id: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Cancel an open auction. No-op on closed / cancelled auctions.
    Cancel {
        /// Auction id.
        id: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}
