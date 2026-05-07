//! `argo duty` clap subcommands (Stream
//! `claude-worker-duty-scheduler`, V1.3). Sibling-file pattern.
//!
//! Surfaces the V1.3 autonomous-loop primitives: list / show /
//! pause / resume / fire-now. Adding a duty is currently done via
//! direct DB seed in tests; an `add` subcommand is intentionally
//! out-of-scope here (tracked as a follow-up brief).
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DutyCmd {
    /// List duties for one tenant in created-at order.
    List {
        /// Tenant id (uuid).
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of the human table.
        #[arg(long)]
        json: bool,
    },
    /// Show one duty by id (uuid).
    Show {
        /// Duty id (uuid).
        id: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Pause a duty until the given UTC instant. Supply
    /// `--until <RFC3339>` for an explicit deadline; without it
    /// the duty pauses 24h from now.
    Pause {
        /// Duty id (uuid).
        id: String,
        /// Resume-at instant (RFC 3339, UTC).
        #[arg(long)]
        until: Option<String>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Resume a paused duty (clears `paused_until`).
    Resume {
        /// Duty id (uuid).
        id: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Fire a duty immediately, ignoring cron + gate. Records the
    /// fire as a `manual_fire` decision in the link table.
    FireNow {
        /// Duty id (uuid).
        id: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}
