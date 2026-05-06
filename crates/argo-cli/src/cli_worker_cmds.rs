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
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum WorkerCmd {
    /// Create a persistent worker instance from a vertical
    /// template, persona name, and delegated role charter.
    Create {
        /// Vertical template id to instantiate.
        #[arg(long)]
        vertical: String,
        /// Public-facing persona name.
        #[arg(long)]
        persona: String,
        /// Role charter delegated to this worker.
        #[arg(long)]
        charter: String,
        /// Operator email or handle that built the worker.
        #[arg(long)]
        builder: Option<String>,
        /// Override the default persona voice preamble.
        #[arg(long)]
        voice_preamble: Option<String>,
        /// Cron-driven duty: `<cron>::<entry_action>::<description>`.
        #[arg(long = "duty")]
        duties: Vec<String>,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Show the full worker identity plus the resolved policy for
    /// one tenant-scoped worker instance.
    Show {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// List worker instances for one tenant, optionally filtered
    /// by persona name.
    List {
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Filter to workers whose persona name exactly matches.
        #[arg(long)]
        persona: Option<String>,
        /// Emit JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Inspect resolved worker auto-approval policy documents.
    #[command(subcommand)]
    Policy(WorkerPolicyCmd),
    /// Update an existing worker's auto-approval policy from a
    /// JSON file (Stream `claude-worker-instance-policy`). The
    /// file must deserialise into `argo_identity::AutoApprovalPolicy`;
    /// when the worker's policy is `enabled = true`, it overrides
    /// the workspace default at gate time.
    Update {
        /// Worker id (UUIDv7) to update.
        worker_id: String,
        /// Path to a JSON `AutoApprovalPolicy` document.
        #[arg(long = "auto-approval-json")]
        auto_approval_json: PathBuf,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Manage actions the worker has proposed / approved /
    /// executed. Backed by `worker_action` memory rows in the
    /// originating job's vertical namespace.
    #[command(subcommand)]
    Actions(WorkerActionsCmd),
    /// Inventory view: aggregated per-`item_id` summary derived
    /// from the action ledger. Operators running a multi-day
    /// worker (e.g. selling skates) use this to answer "what's
    /// in inventory, what's sold, what needs my attention?" in
    /// one shot. Read-only — never writes to the ledger.
    #[command(subcommand)]
    Items(WorkerItemsCmd),
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

#[derive(Subcommand, Debug)]
pub enum WorkerItemsCmd {
    /// List every `item_id` the worker has ever touched in the
    /// action ledger, with per-item rollups: sold / active state,
    /// total actions, count per action-state, latest activity.
    /// Operator's "what's in inventory?" view.
    List {
        /// Filter to a single vertical's namespace.
        #[arg(long)]
        vertical: Option<String>,
        /// Filter by sold status. `sold` shows items with a
        /// completed `mark_item_sold` row. `active` shows items
        /// with no completed mark_item_sold (i.e. still in
        /// inventory). `all` (default) shows both.
        #[arg(long, default_value = "all")]
        state: String,
        /// Cap the number of rows printed. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit JSON-lines (one row per item) instead of a
        /// human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Show the full action history for one item, in time order.
    /// Unlike `actions list --item-id <id>` (which shows the
    /// latest snapshot per action), `items show` walks every
    /// `worker_action` memory row tagged with the given
    /// `item_id` so the operator can audit who approved what,
    /// when it executed, and what the result was. Designed for
    /// "why is skates-A still in inventory?" forensics.
    Show {
        /// The `item_id` to inspect. Exact-match against the
        /// `item_id` field stamped at emission time.
        item_id: String,
        /// Filter to a single vertical's namespace. When the
        /// same item_id exists under multiple verticals (rare
        /// but possible), required to disambiguate.
        #[arg(long)]
        vertical: Option<String>,
        /// Emit JSON (full row dump) instead of the human-
        /// readable timeline. JSON output is one object per row
        /// in chronological order.
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
        /// Filter by `item_id` — the inventory tag stamped onto
        /// each action at emission time. Lets the operator scope
        /// a long-running multi-day worker to "what's happened
        /// for skate-pair-A" without mentally diffing the full
        /// ledger. Matches actions whose `item_id` field equals
        /// the given string exactly; rows with no `item_id` are
        /// excluded.
        #[arg(long)]
        item_id: Option<String>,
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
    /// Cancel a pending or approved action (Stream
    /// `claude-action-cancel-flow`). Records the operator + note
    /// into `result.cancellation`. Refused for executing /
    /// completed / failed / blocked actions — those are terminal
    /// from cancel's standpoint. Cancelled is itself terminal.
    Cancel {
        /// `action_id` (UUIDv7) of the pending or approved action.
        action_id: String,
        /// Operator identifier recorded into the cancellation
        /// payload. Defaults to `$USER` when omitted.
        #[arg(long)]
        cancelled_by: Option<String>,
        /// Optional free-form note recorded alongside the
        /// cancellation timestamp.
        #[arg(long)]
        note: Option<String>,
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
    ///
    /// Retry policy (#755): the bound vertical's manifest can
    /// declare `[actions.<name>.retry] max_attempts = N`. The
    /// OS counts the retry chain from the source's `evidence`
    /// and refuses retries past `max_attempts` unless `--force`
    /// is passed. `--force` overrides log to stderr for audit.
    Retry {
        /// `action_id` (UUIDv7) of the failed action.
        action_id: String,
        /// Bypass the manifest's `max_attempts` ceiling. Logged
        /// to stderr so audit reviewers see when an operator
        /// overrode the policy. Use sparingly — typically the
        /// fix is upstream (different prompt, different
        /// vertical, different time-of-day) rather than another
        /// blind retry.
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
}
