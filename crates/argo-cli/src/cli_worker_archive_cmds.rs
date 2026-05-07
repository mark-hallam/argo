//! `argo worker archive add/list` subcommand schema (Stream
//! `claude-worker-archive-rehydrate`, mission claim 1). Lives in a
//! sibling file so `cli_worker_cmds.rs` stays under the 500-LOC
//! per-file ceiling. Re-exported from `cli_worker_cmds` for
//! continuity with the existing `pub use cli_worker_cmds::WorkerCmd`
//! call sites.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerArchiveCmd {
    /// Archive one live worker. Removes its persona/role/duty rows
    /// from the live tables and stores a single frozen-JSON row in
    /// `worker_archive`. Brief shorthand was `argo worker archive
    /// <id>`; clap can't have a positional alongside subcommands at
    /// the same level, so the verb form is `argo worker archive add
    /// <id>`.
    Add {
        /// Worker id (UUIDv7) to archive.
        worker_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Optional free-text reason recorded alongside the archive
        /// row (e.g. "campaign over", "customer churned").
        #[arg(long)]
        reason: Option<String>,
        /// Operator principal recorded as `archived_by_principal`.
        /// Defaults to `$USER` when omitted.
        #[arg(long)]
        principal: Option<String>,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// List archived workers for one tenant, newest first.
    List {
        /// Tenant id whose archive should be listed.
        #[arg(long)]
        tenant: String,
        /// Emit JSON-lines (one row per archive entry) instead of a
        /// human-readable table.
        #[arg(long)]
        json: bool,
    },
}
