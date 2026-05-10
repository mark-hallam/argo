//! `argo handoff-rules` subcommands (V1.3 —
//! `claude-handoff-routing-rules`). Sibling-file pattern.
//!
//! Note: surfaced as the top-level `handoff-rules` command rather
//! than nested under `argo handoff` because the existing
//! `HandoffCmd` enum lives in `cli_session_cmds.rs` (outside this
//! brief's may-touch list). Functionally equivalent.

use clap::{Args, Subcommand};

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

/// `argo handoff-policy` subcommands (Stream
/// `claude-worker-handoff-policy`). Surfaced as a top-level
/// command for the same reason as `handoff-rules` —
/// `HandoffCmd` lives in `cli_session_cmds.rs` outside the
/// brief's may-touch list.
#[derive(Subcommand, Debug)]
pub enum HandoffPolicyCmd {
    /// Print the parsed `.skycot/handoff-policy.toml`. With
    /// `--explain <action>`, also trace which rule (if any) fires
    /// for that action_kind.
    Show {
        /// Trace the matched rule for this action kind.
        #[arg(long, value_name = "action")]
        explain: Option<String>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Schema-check a handoff-policy file. Defaults to
    /// `.skycot/handoff-policy.toml` under the current dir; pass a
    /// path to validate a different file.
    Validate {
        /// Path to the policy file (defaults to
        /// `.skycot/handoff-policy.toml`).
        path: Option<std::path::PathBuf>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}

/// `argo handoff-stats` arguments (Stream `codex-handoff-stats-cli`).
///
/// Surfaced as the top-level `argo handoff-stats` for the same
/// reason as `handoff-rules` / `handoff-auctions` / `handoff-policy`
/// — the existing `HandoffCmd` enum lives in `cli_session_cmds.rs`,
/// which is outside this brief's may-touch list.
///
/// Two modes:
///   - default: aggregate markdown report (overview + top-N + breakdown)
///   - `--histogram`: ASCII volume histogram, one bar per
///     `--bucket-mins` window
#[derive(Args, Debug)]
pub struct HandoffStatsArgs {
    /// Tenant uuid. Falls back to `ARGO_DEFAULT_TENANT`.
    #[arg(long)]
    pub tenant: Option<String>,
    /// Window size as `<n><unit>` (`s`/`m`/`h`/`d`). Default `7d`
    /// for the report mode and `1d` for `--histogram`.
    #[arg(long)]
    pub since: Option<String>,
    /// Group axis for the breakdown table: `worker` (handed-off-FROM,
    /// the default), `skill` (groups by `expected_outcome` since
    /// handoff rows do not carry a typed skill column), or
    /// `destination` (handed-off-TO, projected to a worker id or
    /// `human:<addr>` label).
    #[arg(long)]
    pub by: Option<String>,
    /// Switch to ASCII volume histogram mode.
    #[arg(long)]
    pub histogram: bool,
    /// Bucket size in minutes for `--histogram`. Default 60 (a
    /// 24-bucket day at the default `--since 1d`). Clamped to
    /// `>= 1`.
    #[arg(long, default_value_t = 60)]
    pub bucket_mins: u32,
    /// Emit a JSON envelope instead of the markdown rendering.
    #[arg(long)]
    pub json: bool,
}
