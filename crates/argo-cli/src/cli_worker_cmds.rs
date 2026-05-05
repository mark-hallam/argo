//! `argo worker` — Worker v3 execution-mode action ledger CLI
//! (#728/#730). Distinct from `argo work {add,serve}` (which
//! manages the queue + the running worker process); `argo
//! worker` is for inspecting and acting on the actions the
//! worker has proposed / approved / executed.
//!
//! #728 shipped read-side (`list` / `show`). #729 added the
//! emission layer. #730 (this PR) ships the write-side state-
//! transition CLI: `approve` / `approve-batch` / `execute` /
//! `complete` / `fail` / `status` / `render`. Each transition
//! appends a NEW `worker_action` memory row carrying the full
//! snapshot at that state — append-only, latest-wins per
//! `action_id`. Validates against `ActionState::can_transition_to`
//! so an `executing` action can't be re-approved, etc.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerCmd {
    /// Manage actions the worker has proposed / approved /
    /// executed. Backed by `worker_action` memory rows in the
    /// originating job's vertical namespace.
    #[command(subcommand)]
    Actions(WorkerActionsCmd),
    /// Print a one-shot SKYCOT health check across the workspace:
    /// verticals loaded (with action-section status), action-ledger
    /// counts by state, recent failures + blocked rows, and the
    /// last-refresh time of the workspace ledger files. Operators
    /// run this to answer "is the worker setup healthy?" without
    /// scripting 5 commands together.
    Doctor {
        /// Filter the ledger view to a single vertical.
        #[arg(long)]
        vertical: Option<String>,
        /// Emit JSON instead of the human-readable report.
        #[arg(long)]
        json: bool,
    },
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
    /// Approve an approval-gated action: `pending → approved`.
    /// Records `approved_by` + `approved_at`. Refuses if the
    /// action isn't currently `pending` or doesn't require
    /// approval (per the manifest classification).
    Approve {
        /// `action_id` (UUIDv7).
        action_id: String,
        /// Operator identifier recorded into `approved_by`.
        /// Defaults to `$USER` when omitted.
        #[arg(long)]
        approved_by: Option<String>,
        /// Optional free-form note appended into the row's
        /// `result` field for audit context.
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Approve multiple actions in one call. Each id is processed
    /// independently — one failure does not stop the rest.
    ApproveBatch {
        /// Space-separated list of `action_id`s.
        action_ids: Vec<String>,
        #[arg(long)]
        approved_by: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Mark an approved (or autonomous-pending) action as
    /// `executing`. The actual executor logic ships in #731 —
    /// for now this is just a state stamp so an operator can
    /// signal "I started doing this offline".
    Execute {
        action_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Mark an executing action as `completed`. Records
    /// `executed_at` and stores any operator-supplied result.
    Complete {
        action_id: String,
        /// Free-form one-liner stored under `result.summary`.
        #[arg(long)]
        note: Option<String>,
        /// Optional structured result blob. Must be valid JSON.
        #[arg(long)]
        result_json: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Mark an executing action as `failed`. Records the reason
    /// into `result.failure` for downstream retry / triage.
    Fail {
        action_id: String,
        /// Free-form failure summary (required).
        #[arg(long)]
        reason: String,
        #[arg(long)]
        json: bool,
    },
    /// Show the full state-row history for one action — every
    /// `worker_action` row that shares the `action_id`, in
    /// time order. Useful to audit who approved when.
    Status {
        action_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Render workspace-level per-state ledger artifacts under
    /// `.skycot/`. Writes `approval_queue.md`,
    /// `completed_actions.md`, and `blocked_actions.md` so the
    /// operator has a single review surface across all jobs.
    Render {
        /// Optional vertical filter — when omitted, walks every
        /// loaded vertical.
        #[arg(long)]
        vertical: Option<String>,
    },
    /// Retry a failed action: emit a NEW pending action with the
    /// same payload + platform + item_id, with the failed
    /// action_id referenced in the new action's `evidence`. The
    /// new action goes through the OS classification fresh — if
    /// the manifest still says `requires_approval = true`, it
    /// lands `pending` awaiting approval. The failed action stays
    /// terminal in the ledger; nothing is mutated retroactively.
    Retry {
        /// `action_id` (UUIDv7) of the failed action.
        action_id: String,
        #[arg(long)]
        json: bool,
    },
}
