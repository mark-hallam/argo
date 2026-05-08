//! clap-derived CLI entrypoint; the top-level `Command` enum lives in
//! `cli_command_enum.rs` so this file stays under the 500-LOC ceiling.
#[path = "cli_reexports.rs"]
mod cli_reexports;
pub use cli_reexports::*;
pub use cli_reexports::{
    cli_duty_cmds, cli_eval_cmds, cli_handoff_cmds, cli_org_cmds, cli_policy_cmds,
    cli_recover_cmds, cli_roster_cmds, cli_skill_ab_cmds, cli_worker_cmds,
};
#[path = "cli_command_enum.rs"]
mod cli_command_enum;
use clap::{Args, Parser, Subcommand};
pub use cli_command_enum::Command;
#[derive(Parser, Debug)]
#[command(
    name = "argo",
    version,
    about = "Argo — the ship of agents",
    long_about = "Argo is the agent framework that one engineer can fully audit on a weekend.\n\
                  See `docs/plan/argo-build-plan.md` for the full master spec.",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum ToolsCmd {
    /// List in-core tools.
    List {
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        #[arg(long, default_value = "text")]
        format: String,
        /// Case-insensitive substring filter on tool name.
        #[arg(long)]
        name_contains: Option<String>,
    },
    /// Show one tool's full JSON-schema parameter shape by name.
    Show {
        name: String,
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ApprovalCmd {
    /// Export approval audit rows to CSV, JSON, or JSONL.
    Export(ApprovalExportArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ComplianceSoc2Args {
    /// Window start, RFC3339. Inclusive.
    #[arg(long)]
    pub from: String,
    /// Window end, RFC3339. Exclusive.
    #[arg(long)]
    pub to: String,
    /// Optional actor id filter.
    #[arg(long)]
    pub actor: Option<String>,
    /// Optional action / kind filter.
    #[arg(long)]
    pub action: Option<String>,
    /// Optional tenant id filter.
    #[arg(long)]
    pub tenant: Option<String>,
    /// Output format: `markdown` (default), `json`, or `csv`.
    #[arg(long, default_value = "markdown")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
pub struct ApprovalExportArgs {
    /// Include approvals requested at or after this time (`30d`, `12h`, or RFC3339).
    #[arg(long)]
    pub since: Option<String>,
    /// Include approvals requested at or before this time (`30d`, `12h`, or RFC3339).
    #[arg(long)]
    pub until: Option<String>,
    /// Filter by decision; may be repeated. Values: approve, deny, pending, timeout.
    #[arg(long)]
    pub decision: Vec<String>,
    /// Strict equality filter on worker id/name.
    #[arg(long)]
    pub worker: Option<String>,
    /// Strict equality filter on action kind.
    #[arg(long = "action-kind")]
    pub action_kind: Option<String>,
    /// Maximum rows to export; clamped to 1000.
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    /// Output format: csv, json, or jsonl.
    #[arg(long, default_value = "csv")]
    pub format: String,
    /// Output path, or `-` for stdout.
    #[arg(long, default_value = "-")]
    pub out: String,
    /// Allow overwriting an existing output file.
    #[arg(long)]
    pub force: bool,
}
