//! Cloud-sandbox CLI surface (Stream `mission-e2b-reap-idle`) — split from
//! `cli_command_enum.rs` so that file stays under the 500-LOC budget. E2B
//! today; AWS/GCP CU sandboxes when those land.

use clap::Subcommand;

/// Cloud-sandbox CLI surface (Stream `mission-e2b-reap-idle`). E2B
/// today; AWS/GCP CU sandboxes when those land.
#[derive(Subcommand, Debug)]
pub enum SandboxCmd {
    /// E2B managed-sandbox ops.
    E2b {
        #[command(subcommand)]
        cmd: E2bSandboxCmd,
    },
}

/// E2B-specific sandbox subcommands.
#[derive(Subcommand, Debug)]
pub enum E2bSandboxCmd {
    /// JSON-line list of all sandboxes the configured E2B API key owns.
    List,
    /// One-shot reap of sandboxes idle longer than `--max-age` seconds.
    /// Emits a single JSON summary object.
    ReapIdle {
        /// Idle-time threshold in seconds. Default 600 (10 minutes).
        #[arg(long, default_value_t = 600)]
        max_age: u64,
        /// List the would-be-reaped sandboxes without closing them.
        #[arg(long)]
        dry_run: bool,
    },
}
