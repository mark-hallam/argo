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
    Validate { manifest: String },
    Test { provider: String },
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
    View { name: String },
    Create { name: String },
    Refine { name: String },
    Install { source: String },
    Lock { name: String },
    Unlock { name: String },
}

#[derive(Subcommand, Debug)]
pub enum MemoryCmd {
    List,
    View { entry: String },
    Edit { entry: String },
    Revert { entry: String },
    Forget { entry: String },
    Lock { entry: String },
    Unlock { entry: String },
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
}

#[derive(Subcommand, Debug)]
pub enum AuditCmd {
    Show,
    /// Verify the hash chain.
    Verify {
        #[arg(long)]
        from: Option<u64>,
        #[arg(long)]
        to: Option<u64>,
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
    /// Show current schema version.
    Status,
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
