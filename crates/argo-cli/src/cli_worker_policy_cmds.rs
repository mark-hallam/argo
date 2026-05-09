//! `argo worker policy` subcommands. Extracted from
//! `cli_worker_cmds.rs` so that file stays under the 500-LOC
//! per-file ceiling (`claude-cli-worker-cmds-split`).
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerPolicyCmd {
    /// Print the resolved AutoApprovalPolicy JSON for one worker.
    Show {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Emit JSON. Accepted for symmetry with every worker subcommand.
        #[arg(long)]
        json: bool,
    },
}
