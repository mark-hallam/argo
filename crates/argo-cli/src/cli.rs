//! clap-derived CLI surface (§19 command tree).
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "argo",
    version,
    about = "Argo — the ship of agents",
    long_about = "Argo is the agent framework that one engineer can fully audit on a weekend.\n\
                  See `docs/plan/argo-build-plan.md` for the full master spec.",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start a fresh session.
    New,
    /// Resume an existing session by id (or interactive picker if omitted).
    Resume {
        id: Option<String>,
    },
    Agents {
        #[command(subcommand)]
        cmd: Option<AgentsCmd>,
    },
    Gateway {
        #[command(subcommand)]
        cmd: GatewayCmd,
    },
    Mcp {
        #[command(subcommand)]
        cmd: McpCmd,
    },
    Acp {
        #[command(subcommand)]
        cmd: AcpCmd,
    },
    Plugin {
        #[command(subcommand)]
        cmd: PluginCmd,
    },
    Provider {
        #[command(subcommand)]
        cmd: ProviderCmd,
    },
    Model {
        #[command(subcommand)]
        cmd: ModelCmd,
    },
    Skill {
        #[command(subcommand)]
        cmd: SkillCmd,
    },
    Memory {
        #[command(subcommand)]
        cmd: MemoryCmd,
    },
    Cron {
        #[command(subcommand)]
        cmd: CronCmd,
    },
    Audit {
        #[command(subcommand)]
        cmd: AuditCmd,
    },
    Workspace {
        #[command(subcommand)]
        cmd: WorkspaceCmd,
    },
    Db {
        #[command(subcommand)]
        cmd: DbCmd,
    },
    Costs {
        #[command(subcommand)]
        cmd: Option<CostsCmd>,
    },
    Session {
        #[command(subcommand)]
        cmd: SessionCmd,
    },
    Forge {
        #[command(subcommand)]
        cmd: ForgeCmd,
    },
    Judge {
        #[command(subcommand)]
        cmd: JudgeCmd,
    },
    Route {
        #[command(subcommand)]
        cmd: RouteCmd,
    },
    Steer {
        session_id: String,
        text: String,
    },
    Trace {
        #[arg(long)]
        turn: Option<String>,
        #[arg(long)]
        session: Option<String>,
    },
    /// Validate an eval task JSONL file (parses every row, reports
    /// per-line errors + the metric histogram).
    Eval {
        #[command(subcommand)]
        cmd: EvalCmd,
    },
    /// Open the interactive TUI (read-only session viewer for now).
    Tui,
    /// Diagnose configuration / sandbox / port issues.
    Doctor,
    Import {
        #[command(subcommand)]
        cmd: ImportCmd,
    },
    Update,
}

#[derive(Subcommand, Debug)]
pub enum AgentsCmd {
    List,
    Create { name: String },
    Delete { name: String },
}

#[derive(Subcommand, Debug)]
pub enum GatewayCmd {
    Start,
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
    List,
    View {
        name: String,
    },
    /// Verify a skill's body matches its frontmatter `provenance.hash`.
    Verify {
        name: String,
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
    List,
    View {
        entry: String,
    },
    /// Append a fresh memory entry. `<file>` is one of memory|user|soul|agents.
    Append {
        file: String,
        body: String,
    },
    /// Verify a memory entry's body matches its stored SHA-256 hash.
    Verify {
        entry: String,
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
    List,
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
    /// Show the kind histogram + (optionally) the last `--tail N` rows.
    Show {
        #[arg(long)]
        tail: Option<u32>,
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
    },
    /// Stream the audit log to ndjson. Defaults to stdout; pass
    /// `--out <path>` to write to a file.
    Export {
        #[arg(long, default_value = "ndjson")]
        format: String,
        #[arg(long)]
        out: Option<String>,
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
    /// Show the configured caps.
    Show,
}

#[derive(Subcommand, Debug)]
pub enum SessionCmd {
    /// List recent sessions for the current tenant.
    List {
        #[arg(long, default_value_t = 25)]
        limit: u32,
    },
    /// Show one session header (model, provider, start time, message count).
    Show {
        id: String,
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
    Replay {
        id: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        tools: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
    },
    /// Structural diff of two recorded sessions.
    Diff {
        id1: String,
        id2: String,
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
