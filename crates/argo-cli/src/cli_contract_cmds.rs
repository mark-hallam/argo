//! `argo contract apply/validate/show` subcommand schema (Stream
//! `claude-delegation-contract-template`, V1.1 mission claim 2).
//! Lives in a sibling file so `cli_command_enum.rs` stays under the
//! 500-LOC per-file ceiling — same pattern used by
//! `cli_worker_okr_cmds.rs`.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ContractCmd {
    /// Materialise a `<role>.contract.toml` file into an existing
    /// worker (charter + KPI definitions + objectives + KRs). Use
    /// `--dry-run` to preview the planned mutations without writing.
    Apply {
        /// Path to the `<role>.contract.toml` file.
        file: PathBuf,
        /// Tenant id whose worker namespace should receive the contract.
        #[arg(long)]
        tenant: Option<String>,
        /// Worker id (UUIDv7) — required unless `--dry-run`.
        #[arg(long = "worker")]
        worker: Option<String>,
        /// Print the planned mutations without persisting anything.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Schema-check only — parses the file, runs semantic validation,
    /// and reports counts. Never touches the store.
    Validate {
        /// Path to the `<role>.contract.toml` file.
        file: PathBuf,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Render a worker's current state as TOML so it round-trips with
    /// `apply`. Reads charter + kpis blob + objectives/KRs.
    Show {
        /// Worker id (UUIDv7) to read.
        worker_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of TOML.
        #[arg(long)]
        json: bool,
    },
}
