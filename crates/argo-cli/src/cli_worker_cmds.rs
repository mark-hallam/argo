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
        /// Per-worker filesystem-executor sandbox root. Defaults to
        /// `/tmp/argo-fs/<worker_id>/` when omitted (V1.2 brief
        /// `claude-filesystem-executor`).
        #[arg(long = "fs-root")]
        fs_root: Option<PathBuf>,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Create a worker from a single TOML persona/role spec.
    Onboard {
        /// Path to the onboarding TOML spec.
        #[arg(long = "persona-spec")]
        persona_spec: PathBuf,
        /// Explicit worker id. Fails if it already exists.
        #[arg(long)]
        id: Option<String>,
        /// Emit JSON instead of a markdown summary.
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
    /// Show cost attribution from the worker budget ledger.
    Costs(WorkerCostsCmd),
    /// Show live worker presence ordered by `last_active_at DESC`,
    /// with workers idle >30 days flagged `idle` and never-seen
    /// rows rendered as `(never seen)` (Stream
    /// `claude-worker-presence-tracking`).
    Fleet {
        /// Optional tenant id filter. Omitted means all tenants.
        #[arg(long)]
        tenant: Option<String>,
        /// Cap the number of rows printed. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Only include workers active since this duration
        /// (`7d`, `12h`, `30m`, or `60s`).
        #[arg(long)]
        since: Option<String>,
        /// Emit JSON instead of a markdown table.
        #[arg(long)]
        json: bool,
    },
    /// Render a paste-friendly one-page worker portrait.
    Portrait {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of markdown.
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
    /// Resume a worker whose circuit breaker is open. Moves it
    /// to half-open so the next action is a supervised probe.
    Resume {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace should be resumed.
        #[arg(long)]
        tenant: String,
        /// Operator note recorded in the circuit audit trail.
        #[arg(long)]
        reason: Option<String>,
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
    /// List recent behaviour anomaly events for one worker
    /// (Stream `claude-anomaly-detector`). Each row shows
    /// `dimension / observed / z-score` so the operator can
    /// spot drift the circuit breaker doesn't see (subtler
    /// latency spikes, retry-count creep, etc.).
    Anomalies {
        /// Worker id (UUIDv7 or `nil` for the platform-default
        /// worker that runs unbound work).
        worker_id: String,
        /// Filter to events observed within this many recent
        /// minutes. When unset, returns the most-recent N rows.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Cap the number of rows printed. Default 100 (max 1000).
        #[arg(long)]
        limit: Option<u32>,
        /// Emit JSON-lines (one event per row) instead of a
        /// human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Manage the per-worker charter (markdown delegation contract,
    /// ≤ 16 KB) and KPIs (free-form JSON, ≤ 8 KB) — backs mission
    /// claim 2 ("Workers operate under a written charter").
    #[command(subcommand)]
    Charter(WorkerCharterCmd),
    /// Sleep / wake a worker — freezes the full identity into a
    /// `worker_archive` row and removes the live persona/role/duty
    /// rows. Audit + handoff history stay intact (Stream
    /// `claude-worker-archive-rehydrate`, mission claim 1).
    #[command(subcommand)]
    Archive(WorkerArchiveCmd),
    /// Inverse of `worker archive` — restore a previously-archived
    /// worker by re-creating its persona/role/duty rows from the
    /// frozen JSON snapshot.
    Rehydrate {
        /// Worker id (UUIDv7) of the archived worker.
        worker_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Add, remove, or list the worker's notification channels
    /// (`Stream claude-worker-channels-cli`). Channels live as JSON
    /// inside the worker row — no schema migration. Cap of 16 per
    /// worker; `add` is idempotent on `<kind>:<identifier>` dup.
    #[command(subcommand)]
    Channels(crate::cli::cli_worker_channels_cmds::WorkerChannelsCmd),
    /// Set or inspect the worker's daily / monthly spend caps
    /// (Stream `claude-worker-budget-cli`). Caps land on the
    /// worker's auto-approval policy JSON; period-to-date spend
    /// rolls up the existing `worker_budget` ledger.
    #[command(subcommand)]
    Budget(crate::cli::cli_worker_budget_cmds::WorkerBudgetCmd),
    /// Record / list / aggregate observed KPI values for one
    /// worker (Stream `claude-worker-kpi-tracker`, mission claim 2
    /// runtime evidence). Charter storage records KPI *definitions*
    /// on the worker row; this is the time-series store of observed
    /// numeric values.
    #[command(subcommand)]
    Kpi(WorkerKpiCmd),
    /// Set / show / score quarterly OKRs for one worker
    /// (Stream `claude-worker-okr-cli`, mission claim 2 goals layer).
    /// KPI tracker records observed values; OKRs declare intent
    /// against named KPIs and score progress.
    #[command(subcommand)]
    Okr(WorkerOkrCmd),
    /// Quarterly / monthly / weekly review packet bundling KPI
    /// deltas, success rate, operator feedback, handoffs, and
    /// anomalies for one worker (Stream
    /// `claude-worker-review-cycle-cli`, mission claim 2). Read-only
    /// aggregator — no new schema.
    Review {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Period preset: `quarterly` (default), `monthly`, `weekly`.
        #[arg(long, default_value = "quarterly")]
        period: String,
        /// Override the period start with an explicit RFC3339 cutoff.
        #[arg(long)]
        since: Option<String>,
        /// `markdown` (default) or `json`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Manage declared inter-worker dependencies with per-kind cycle
    /// detection (Stream `claude-worker-dependency-tracking`,
    /// V1.1). Distinct from `handoff_routing` runtime weights;
    /// this is *declared intent* — what the operator says one
    /// worker structurally needs from another.
    #[command(subcommand)]
    Deps(crate::cli::cli_worker_deps_cmds::WorkerDepsCmd),
    /// Per-worker auto-approval success rollup over typed `AuditKind`
    /// approval kinds. Read-only join over `audit_record` +
    /// `session.worker_id` (mission claim 2 KPI seed).
    Success {
        worker_id: String,
        /// `now - <duration>` filter; suffixes `s`/`m`/`h`/`d`.
        #[arg(long)]
        since: Option<String>,
        /// `none` (default), `day`, `week`, or `month`.
        #[arg(long, default_value = "none")]
        bucket: String,
        /// `markdown` (default) or `json`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Promote a worker up the `entry → junior → senior → lead`
    /// tenure ladder (Stream `claude-worker-promotion-workflow`,
    /// mission claim 2). Without `--to`, advances to the next level.
    /// `--force` overrides the criteria gate (sessions / success-rate
    /// / tenure-days); promotion to `lead` always requires `--force`.
    Promote {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Explicit target level (`junior` / `senior` / `lead`). When
        /// omitted, the worker is advanced to the next level above
        /// its current one.
        #[arg(long)]
        to: Option<String>,
        /// Bypass the criteria gate (records the override + reason in
        /// the audit row). Required for `--to lead` regardless of
        /// numeric thresholds.
        #[arg(long)]
        force: bool,
        /// Operator note recorded in the promotion audit row.
        #[arg(long)]
        reason: Option<String>,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Show the worker's current level and the numeric gap to the
    /// next promotion (e.g. "23 sessions short, 4d tenure short").
    /// Read-only.
    Level {
        /// Worker id (UUIDv7).
        worker_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
}

// Leaf enums live in sibling files so this module stays well below
// the 500-LOC per-file ceiling and has headroom for future
// `argo worker <verb>` extensions. Re-exported below for the
// corresponding `WorkerCmd` variants. The budget enum is reached
// through its sibling path directly — this dispatcher hands the
// whole cmd to `handlers::worker_budget::run` rather than
// pattern-matching here.
pub use crate::cli::cli_worker_actions_cmds::WorkerActionsCmd;
pub use crate::cli::cli_worker_archive_cmds::WorkerArchiveCmd;
pub use crate::cli::cli_worker_charter_cmds::WorkerCharterCmd;
pub use crate::cli::cli_worker_costs_cmds::WorkerCostsCmd;
pub use crate::cli::cli_worker_items_cmds::WorkerItemsCmd;
pub use crate::cli::cli_worker_kpi_cmds::WorkerKpiCmd;
pub use crate::cli::cli_worker_okr_cmds::WorkerOkrCmd;
pub use crate::cli::cli_worker_policy_cmds::WorkerPolicyCmd;
