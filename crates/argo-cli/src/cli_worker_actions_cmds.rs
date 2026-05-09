//! `argo worker actions` subcommands. Extracted from
//! `cli_worker_cmds.rs` so that file stays under the 500-LOC
//! per-file ceiling (`claude-cli-worker-cmds-split`).
use clap::Subcommand;

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
    /// Apply one decision (approve, or deny via `--deny`) to
    /// every listed `action_id`. Each id processes independently;
    /// one failure doesn't abort the rest. Confirms before
    /// writing unless `--confirm` (`-y`) is set, or `--dry-run`
    /// lists the ids without writing
    /// (`claude-approval-batch-decisions-cli`).
    ApproveBatch {
        /// Space-separated list of `action_id`s.
        action_ids: Vec<String>,
        #[arg(long)]
        approved_by: Option<String>,
        #[arg(long)]
        note: Option<String>,
        /// Cancel each pending action instead of approving.
        #[arg(long)]
        deny: bool,
        /// Skip the interactive confirmation prompt.
        #[arg(long, short = 'y')]
        confirm: bool,
        /// Preview ids without writing.
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    /// Mark an approved (or autonomous-pending) action as
    /// `executing`. The actual executor logic ships in #731 —
    /// for now this is just a state stamp so an operator can
    /// signal "I started doing this offline".
    Execute {
        action_id: String,
        /// Preview execution without changing state or writing an output bundle.
        #[arg(long)]
        dry_run: bool,
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
