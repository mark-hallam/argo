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
use clap::{Parser, Subcommand};
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
