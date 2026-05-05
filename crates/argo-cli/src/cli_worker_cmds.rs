//! `argo worker` — Worker v3 execution-mode action ledger CLI
//! (#728). Distinct from `argo work {add,serve}` (which manages
//! the queue + the running worker process); `argo worker` is for
//! inspecting and (in later PRs) acting on the actions the
//! worker has proposed / approved / executed.
//!
//! #728 ships only the read-side: `list` and `show`. The
//! write-side (`approve`, `execute`, `complete`) lands in #730
//! once the action types it gates are also in (#729).

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerCmd {
    /// Manage actions the worker has proposed / approved /
    /// executed. Backed by `worker_action` memory rows in the
    /// originating job's vertical namespace.
    #[command(subcommand)]
    Actions(WorkerActionsCmd),
}

#[derive(Subcommand, Debug)]
pub enum WorkerActionsCmd {
    /// List actions, with optional state/job/vertical/type/
    /// platform filters. The "current state" of an action is
    /// the latest persisted row matching its `action_id`.
    List {
        /// Filter by action state. One of `pending`, `approved`,
        /// `executing`, `completed`, `failed`, `blocked`, `all`.
        /// Defaults to `all`.
        #[arg(long, default_value = "all")]
        state: String,
        /// Filter to actions originating from this job_id.
        #[arg(long)]
        job_id: Option<String>,
        /// Filter to actions in a specific vertical's namespace.
        /// When omitted, walks every loaded vertical.
        #[arg(long)]
        vertical: Option<String>,
        /// Filter by `action_type` (e.g. `post_listing`,
        /// `verify_price`). Matches the canonical name; see
        /// `handlers::worker_actions::ActionType` for the full
        /// list.
        #[arg(long)]
        action_type: Option<String>,
        /// Filter by destination platform (e.g. `facebook_marketplace`,
        /// `ebay`, `gumtree`). Optional — most action types are
        /// platform-agnostic.
        #[arg(long)]
        platform: Option<String>,
        /// Cap the number of rows printed. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit JSON-lines (one row per action) instead of a
        /// human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Show the full latest snapshot of one action.
    Show {
        /// `action_id` (UUIDv7) — the stable identifier across
        /// all state transitions.
        action_id: String,
        /// Emit JSON instead of the human-readable preview.
        #[arg(long)]
        json: bool,
    },
}
