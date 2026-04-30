//! Per-family clap `Subcommand` enums. Hoisted out of `cli.rs`
//! to keep both files under the 500-LOC ceiling. Re-exported
//! wholesale by `cli.rs` so handler imports (`use crate::cli::FooCmd;`)
//! continue to work unchanged.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum AgentsCmd {
    List,
    Create { name: String },
    Delete { name: String },
}

#[derive(Subcommand, Debug)]
pub enum GatewayCmd {
    /// Bind a TCP listener and serve the JSON HTTP gateway. The
    /// first cut answers `GET /health` + `GET /version`; the
    /// `Adapter` routing surface from `argo-gateway` lands later.
    Start {
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        #[arg(long, default_value_t = 7400)]
        port: u16,
        /// Stop after N requests (mainly for tests; omit to run
        /// until SIGINT / SIGTERM).
        #[arg(long)]
        max_requests: Option<u64>,
    },
    Stop,
    Status,
}

#[derive(Subcommand, Debug)]
pub enum McpCmd {
    Serve,
}

#[derive(Subcommand, Debug)]
pub enum AcpCmd {
    Serve,
}

#[derive(Subcommand, Debug)]
pub enum PluginCmd {
    List,
    Install { source: String },
    Enable { name: String },
    Disable { name: String },
    Remove { name: String },
    Run { name: String },
}

#[derive(Subcommand, Debug)]
pub enum ProviderCmd {
    List,
    /// Render one provider manifest's full details (header, auth,
    /// transport, capabilities, models, discovery).
    Show {
        id: String,
        /// Output the parsed manifest as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    Validate {
        manifest: String,
    },
    /// Validate every manifest under `manifests/providers/`. Exits
    /// non-zero if any fails parsing — wire into CI to catch
    /// drift early.
    ValidateAll,
    Test {
        provider: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ModelCmd {
    List,
    Set { model: String },
    Test { model: String },
}

#[derive(Subcommand, Debug)]
pub enum SkillCmd {
    List {
        /// Output as JSON (one object per skill) for scripting.
        #[arg(long)]
        json: bool,
    },
    View {
        name: String,
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Verify a skill's body matches its frontmatter `provenance.hash`.
    /// Pass `--all` to verify every active skill in the workspace.
    Verify {
        #[arg(required_unless_present = "all")]
        name: Option<String>,
        /// Verify every active skill instead of one named skill.
        #[arg(long, conflicts_with = "name")]
        all: bool,
    },
    Create {
        name: String,
    },
    Refine {
        name: String,
    },
    Install {
        source: String,
    },
    Lock {
        name: String,
    },
    Unlock {
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum MemoryCmd {
    List {
        /// Output as JSON (one object per entry) for scripting.
        #[arg(long)]
        json: bool,
        /// Limit to one file: memory|user|soul|agents.
        #[arg(long)]
        file: Option<String>,
    },
    View {
        entry: String,
        /// Output the entry as a single JSON object (incl. body).
        #[arg(long)]
        json: bool,
    },
    /// Append a fresh memory entry. `<file>` is one of memory|user|soul|agents.
    Append {
        file: String,
        body: String,
    },
    /// Verify a memory entry's body matches its stored SHA-256 hash.
    /// Pass `--all` to verify every active entry across all four
    /// memory files in the workspace.
    Verify {
        #[arg(required_unless_present = "all")]
        entry: Option<String>,
        /// Verify every active memory entry instead of one named entry.
        #[arg(long, conflicts_with = "entry")]
        all: bool,
    },
    Edit {
        entry: String,
    },
    Revert {
        entry: String,
    },
    Forget {
        entry: String,
    },
    Lock {
        entry: String,
    },
    Unlock {
        entry: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum CronCmd {
    /// List scheduled jobs in the workspace's `cron/jobs/` directory.
    List {
        /// Output as JSON (one object per job) for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Create a new job (`<name> <schedule> <prompt> <platform> <chat>`).
    Create {
        name: String,
        schedule: String,
        prompt: String,
        platform: String,
        chat: String,
    },
    Edit {
        name: String,
    },
    /// Delete a job by name.
    Delete {
        name: String,
    },
    /// Trigger a job immediately (mainly for tests / manual reruns).
    Trigger {
        name: String,
    },
    /// Run one scheduler tick at the current time. Reports which
    /// jobs would fire; pass `--dry-run` to skip writing the
    /// last-fired cursor.
    Tick {
        #[arg(long)]
        dry_run: bool,
    },
    /// Run the cron tick loop in the foreground at a fixed
    /// interval, holding `cron/.tick.lock` so multiple daemons
    /// don't double-fire. Exits with a non-zero status if the
    /// lock is already held.
    Daemon {
        /// Interval between ticks (seconds). Default 60s.
        #[arg(long, default_value_t = 60)]
        interval_secs: u64,
        /// Stop after N ticks (mainly for tests / manual checks).
        /// Omit to run until SIGINT.
        #[arg(long)]
        max_ticks: Option<u64>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuditCmd {
    /// Show the kind histogram + (optionally) the last `--tail N`
    /// rows. Pass `--session <id>` to print every audit row for one
    /// session in `seq` order instead of the global histogram view.
    Show {
        #[arg(long)]
        tail: Option<u32>,
        /// Output as a single JSON object (kind histogram + tail).
        #[arg(long)]
        json: bool,
        /// Session id to scope to. With this set, the output is the
        /// session's full audit chain (seq order, oldest first).
        #[arg(long)]
        session: Option<String>,
        /// Comma-separated list of audit kinds to include (case-
        /// sensitive label match, e.g. `ToolCall,ToolResult`). When
        /// set, the tail / per-session view drops every other kind.
        #[arg(long)]
        kinds: Option<String>,
    },
    /// Verify the hash chain. Pass `--session <id>` to check one
    /// session only (fast on huge logs).
    Verify {
        #[arg(long)]
        from: Option<u64>,
        #[arg(long)]
        to: Option<u64>,
        #[arg(long)]
        session: Option<String>,
        /// Output as a single JSON object (ok / rows / breaks).
        #[arg(long)]
        json: bool,
    },
    /// Stream the audit log to ndjson. Defaults to stdout; pass
    /// `--out <path>` to write to a file. Pass `--session <id>` to
    /// scope the export to one session, or `--since-minutes <N>`
    /// to scope to recent rows.
    Export {
        #[arg(long, default_value = "ndjson")]
        format: String,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        session: Option<String>,
        /// Filter to rows created within this many recent minutes.
        /// Combined with `--session` if both are set.
        #[arg(long)]
        since_minutes: Option<i64>,
    },
}

#[derive(Subcommand, Debug)]
pub enum WorkspaceCmd {
    /// Initialize a fresh workspace at `.` (writes argo.toml + the
    /// canonical bundle directories).
    Init,
    /// One-line status summary (workspace + db + costs).
    Status,
    /// Verify workspace integrity: argo.toml parses, memory files +
    ///   bundle dirs exist, agent persona files exist, manifests
    ///   referenced from `[provider.profiles.*]` resolve.
    Verify,
    Fork {
        source: String,
    },
    Push {
        oci_ref: String,
    },
    Pull {
        oci_ref: String,
    },
    Diff {
        other: String,
    },
    Publish,
}

#[derive(Subcommand, Debug)]
pub enum DbCmd {
    /// Migrate up to latest.
    MigrateUp,
    /// Migrate down to a specific version.
    MigrateDown {
        #[arg(long)]
        to: u32,
    },
    /// Show current schema version + per-table row counts.
    Status,
    /// Drop sessions whose `started_at` is older than `--older-than-days`
    /// days. Pass `--force` to actually delete; otherwise dry-runs.
    Prune {
        #[arg(long, default_value_t = 90)]
        older_than_days: i64,
        #[arg(long)]
        force: bool,
    },
    /// Run SQLite `VACUUM` to reclaim file-space after a prune.
    Vacuum,
}

#[derive(Subcommand, Debug)]
pub enum CostsCmd {
    /// Set a hard cap (`--session`, `--day`, or `--month`) in USD.
    Cap {
        #[arg(long, conflicts_with_all = ["day", "month"])]
        session: bool,
        #[arg(long, conflicts_with_all = ["session", "month"])]
        day: bool,
        #[arg(long, conflicts_with_all = ["session", "day"])]
        month: bool,
        usd: f64,
    },
    /// Show the configured caps + actual-spend roll-up.
    Show {
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
        /// Group spend by `(provider, model)` and print the
        /// top spenders instead of the default ceilings view.
        #[arg(long, conflicts_with = "by_session")]
        by_model: bool,
        /// Group spend by `session_id` and print the top
        /// spenders instead of the default ceilings view.
        #[arg(long)]
        by_session: bool,
    },
    /// Dump every row in `spend_ledger` as NDJSON (one JSON object
    /// per line). Pass `--out <path>` to write to a file; otherwise
    /// streams to stdout. `--since-minutes` scopes to recent rows.
    Export {
        #[arg(long)]
        out: Option<String>,
        /// Filter to rows charged within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
    },
}

#[derive(Subcommand, Debug)]
pub enum SessionCmd {
    /// List recent sessions for the current tenant.
    List {
        #[arg(long, default_value_t = 25)]
        limit: u32,
        /// Output as JSON (one object per session) for scripting.
        #[arg(long)]
        json: bool,
        /// Filter to sessions started within this many recent
        /// minutes (e.g. `--since 60` for the last hour). Compared
        /// against `started_at`.
        #[arg(long)]
        since_minutes: Option<i64>,
    },
    /// Show one session header (model, provider, start time, message count).
    Show {
        id: String,
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Dump a session's messages to JSONL (defaults to stdout;
    /// pass `--out <path>` to write to a file).
    Export {
        id: String,
        #[arg(long)]
        out: Option<String>,
    },
    /// Full-text search the message corpus via FTS5. Pass
    /// `--session <id>` to scope to one session.
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        session: Option<String>,
    },
    /// Delete a session and all its dependent rows. Pass `--force`
    /// to skip the confirmation prompt.
    Delete {
        id: String,
        #[arg(long)]
        force: bool,
    },
    /// Replay a recorded session's user messages into a fresh
    /// session against a (possibly different) model. Each user
    /// message is appended in order and the agent loop runs one
    /// turn per message; the new session id is printed at the end.
    Replay {
        id: String,
        /// Override the model. Defaults to the source session's
        /// model.
        #[arg(long)]
        model: Option<String>,
        /// Override the provider id (matches `manifests/providers/
        /// <id>.toml`). Defaults to the source session's provider.
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        tools: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
    },
    /// Structural diff of two recorded sessions.
    Diff {
        id1: String,
        id2: String,
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
    Bisect {
        id: String,
        hypothesis: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ForgeCmd {
    Optimize {
        skill: String,
        #[arg(long)]
        examples: String,
        #[arg(long, default_value_t = 12)]
        iterations: u32,
        #[arg(long, default_value_t = 5.0)]
        budget: f64,
    },
    Bench {
        skill: String,
    },
    Promote {
        skill: String,
        #[arg(long)]
        version: String,
    },
    Runs {
        #[command(subcommand)]
        cmd: ForgeRunsCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum ForgeRunsCmd {
    List,
    Show { id: String },
    Abort { id: String },
}

#[derive(Subcommand, Debug)]
pub enum JudgeCmd {
    Verdict {
        id: String,
    },
    Override {
        id: String,
        #[arg(long)]
        to: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum RouteCmd {
    Inspect { session_id: String },
    Bench,
}

#[derive(Subcommand, Debug)]
pub enum ImportCmd {
    Hermes { path: String },
}

#[derive(Subcommand, Debug)]
pub enum EvalCmd {
    /// Validate an eval task JSONL file: parse every row, report
    /// per-line errors and the metric histogram.
    Validate { path: String },
}
