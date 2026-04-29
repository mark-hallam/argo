//! `argo` — the user-facing CLI binary.
//!
//! See `docs/plan/argo-build-plan.md` §19 for the full command tree.
//!
//! v0.2 Phase 0/1 scope: working `--version` and `--help` plus stubs for every
//! subcommand documented in §19. Real implementations land per the phase plan.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

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
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start a fresh session.
    New,
    /// Resume an existing session by id (or interactive picker if omitted).
    Resume {
        /// Session id (full UUID or unambiguous prefix).
        id: Option<String>,
    },
    /// Manage named agents within the current tenant.
    Agents {
        #[command(subcommand)]
        cmd: Option<AgentsCmd>,
    },
    /// Run as the messaging gateway daemon.
    Gateway {
        #[command(subcommand)]
        cmd: GatewayCmd,
    },
    /// Run as an MCP server.
    Mcp {
        #[command(subcommand)]
        cmd: McpCmd,
    },
    /// Run as an ACP server (VS Code / Zed / JetBrains).
    Acp {
        #[command(subcommand)]
        cmd: AcpCmd,
    },
    /// Manage plugins.
    Plugin {
        #[command(subcommand)]
        cmd: PluginCmd,
    },
    /// Manage providers.
    Provider {
        #[command(subcommand)]
        cmd: ProviderCmd,
    },
    /// Manage models.
    Model {
        #[command(subcommand)]
        cmd: ModelCmd,
    },
    /// Manage skills.
    Skill {
        #[command(subcommand)]
        cmd: SkillCmd,
    },
    /// Manage memory entries.
    Memory {
        #[command(subcommand)]
        cmd: MemoryCmd,
    },
    /// Manage scheduled tasks.
    Cron {
        #[command(subcommand)]
        cmd: CronCmd,
    },
    /// Inspect or verify the audit log.
    Audit {
        #[command(subcommand)]
        cmd: AuditCmd,
    },
    /// Manage workspace bundles.
    Workspace {
        #[command(subcommand)]
        cmd: WorkspaceCmd,
    },
    /// Database maintenance.
    Db {
        #[command(subcommand)]
        cmd: DbCmd,
    },
    /// Inspect cost ceilings and current spend.
    Costs {
        #[command(subcommand)]
        cmd: Option<CostsCmd>,
    },
    /// Replay a past session against a different model/toolset/workspace.
    Session {
        #[command(subcommand)]
        cmd: SessionCmd,
    },
    /// Run an Argo Forge optimization.
    Forge {
        #[command(subcommand)]
        cmd: ForgeCmd,
    },
    /// Inspect or override judge verdicts.
    Judge {
        #[command(subcommand)]
        cmd: JudgeCmd,
    },
    /// Inspect cost-router decisions.
    Route {
        #[command(subcommand)]
        cmd: RouteCmd,
    },
    /// Inject a steer into a running session.
    Steer {
        /// Target session id.
        session_id: String,
        /// Text to inject.
        text: String,
    },
    /// Inspect a single turn's tracer spans.
    Trace {
        /// Specific turn id.
        #[arg(long)]
        turn: Option<String>,
        /// Specific session id.
        #[arg(long)]
        session: Option<String>,
    },
    /// Diagnose configuration / sandbox / port issues.
    Doctor,
    /// Import settings, sessions, skills, memory from a Hermes installation.
    Import {
        #[command(subcommand)]
        cmd: ImportCmd,
    },
    /// Update to the latest released version.
    Update,
}

#[derive(Subcommand, Debug)]
enum AgentsCmd {
    List,
    Create { name: String },
    Delete { name: String },
}

#[derive(Subcommand, Debug)]
enum GatewayCmd {
    Start,
    Stop,
    Status,
}

#[derive(Subcommand, Debug)]
enum McpCmd {
    Serve,
}

#[derive(Subcommand, Debug)]
enum AcpCmd {
    Serve,
}

#[derive(Subcommand, Debug)]
enum PluginCmd {
    List,
    Install { source: String },
    Enable { name: String },
    Disable { name: String },
    Remove { name: String },
    Run { name: String },
}

#[derive(Subcommand, Debug)]
enum ProviderCmd {
    List,
    Validate { manifest: String },
    Test { provider: String },
}

#[derive(Subcommand, Debug)]
enum ModelCmd {
    List,
    Set { model: String },
    Test { model: String },
}

#[derive(Subcommand, Debug)]
enum SkillCmd {
    List,
    View { name: String },
    Create { name: String },
    Refine { name: String },
    Install { source: String },
    Lock { name: String },
    Unlock { name: String },
}

#[derive(Subcommand, Debug)]
enum MemoryCmd {
    List,
    View { entry: String },
    Edit { entry: String },
    Revert { entry: String },
    Forget { entry: String },
    Lock { entry: String },
    Unlock { entry: String },
}

#[derive(Subcommand, Debug)]
enum CronCmd {
    List,
    Create { name: String },
    Edit { name: String },
    Delete { name: String },
    Trigger { name: String },
}

#[derive(Subcommand, Debug)]
enum AuditCmd {
    Show,
    Verify {
        #[arg(long)]
        from: Option<u64>,
        #[arg(long)]
        to: Option<u64>,
    },
    Export {
        #[arg(long, default_value = "ndjson")]
        format: String,
    },
}

#[derive(Subcommand, Debug)]
enum WorkspaceCmd {
    Init,
    Fork { source: String },
    Push { oci_ref: String },
    Pull { oci_ref: String },
    Diff { other: String },
    Publish,
}

#[derive(Subcommand, Debug)]
enum DbCmd {
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
enum CostsCmd {
    Cap {
        #[arg(long, conflicts_with_all = ["day", "month"])]
        session: bool,
        #[arg(long, conflicts_with_all = ["session", "month"])]
        day: bool,
        #[arg(long, conflicts_with_all = ["session", "day"])]
        month: bool,
        usd: f64,
    },
}

#[derive(Subcommand, Debug)]
enum SessionCmd {
    Replay {
        id: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        tools: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
    },
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
enum ForgeCmd {
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
enum ForgeRunsCmd {
    List,
    Show { id: String },
    Abort { id: String },
}

#[derive(Subcommand, Debug)]
enum JudgeCmd {
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
enum RouteCmd {
    Inspect { session_id: String },
    Bench,
}

#[derive(Subcommand, Debug)]
enum ImportCmd {
    Hermes { path: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        None => println!(
            "argo {} — interactive chat (not yet implemented in Phase 0/1)",
            env!("CARGO_PKG_VERSION")
        ),
        Some(cmd) => not_yet_implemented(&format!("{cmd:?}")),
    }
}

fn not_yet_implemented(what: &str) {
    println!("argo: '{what}' is not yet implemented in this phase.");
    println!("See ROADMAP.md for the phase plan.");
    std::process::exit(2);
}
