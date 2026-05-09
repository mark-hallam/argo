//! `argo worker channels` sub-enum (Stream `claude-worker-channels-cli`).
//!
//! Lives as a sibling of `cli_worker_cmds.rs` so the parent stays under
//! the 500-LOC project ceiling. `WorkerCmd::Channels` references this
//! type via `crate::cli::cli_worker_channels_cmds::WorkerChannelsCmd`.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerChannelsCmd {
    /// Append a `<kind>:<identifier>` channel spec. Idempotent on
    /// exact (kind, target) match — second add prints a warning and
    /// makes no write. Refused past the per-worker cap of 16.
    Add {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// `<kind>:<identifier>` (e.g. `email:ops@example.com`,
        /// `slack:C0123`, `whatsapp:+15551234567`).
        spec: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Remove a channel by `<kind>:<identifier>`. Errors if no
    /// matching channel is present on the worker.
    Remove {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// `<kind>:<identifier>` to remove (must match an existing
        /// channel on the worker). `risk_level` is ignored for
        /// matching purposes.
        channel_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Print the worker's channel list.
    List {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of a markdown bullet list.
        #[arg(long)]
        json: bool,
    },
}
