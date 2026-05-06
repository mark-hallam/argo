//! clap-derived CLI surface (§19 command tree). The top-level
//! `Cli` + `Command` enums live here; per-family `Subcommand` enums
//! are extracted to `cli_subcommands.rs` (re-exported below) so
//! both files stay under the 500-LOC ceiling.
#[path = "cli_action_cmds.rs"]
mod cli_action_cmds;
#[path = "cli_activity_cmds.rs"]
mod cli_activity_cmds;
#[path = "cli_audit_cmds.rs"]
mod cli_audit_cmds;
#[path = "cli_budget_cmds.rs"]
mod cli_budget_cmds;
#[path = "cli_doctor_cmds.rs"]
mod cli_doctor_cmds;
#[path = "cli_eval_cmds.rs"]
pub(crate) mod cli_eval_cmds;
#[path = "cli_forge_cmds.rs"]
mod cli_forge_cmds;
#[path = "cli_gateway_cmds.rs"]
mod cli_gateway_cmds;
#[path = "cli_genesis_cmds.rs"]
mod cli_genesis_cmds;
#[path = "cli_memory_cmd.rs"]
mod cli_memory_cmd;
#[path = "cli_messages_cmds.rs"]
mod cli_messages_cmds;
#[path = "cli_session_cmds.rs"]
mod cli_session_cmds;
#[path = "cli_skill_conflicts_cmds.rs"]
mod cli_skill_conflicts_cmds;
#[path = "cli_skill_review_cmds.rs"]
mod cli_skill_review_cmds;
#[path = "cli_skill_search_cmds.rs"]
mod cli_skill_search_cmds;
#[path = "cli_handoff_cmds.rs"]
pub mod cli_handoff_cmds;
#[path = "cli_org_cmds.rs"]
pub mod cli_org_cmds;
#[path = "cli_policy_cmds.rs"]
pub mod cli_policy_cmds;
#[path = "cli_roster_cmds.rs"]
pub mod cli_roster_cmds;
#[path = "cli_skill_ab_cmds.rs"]
pub mod cli_skill_ab_cmds;
#[path = "cli_subcommands.rs"]
mod cli_subcommands;
#[path = "cli_subcommands_cron.rs"]
mod cli_subcommands_cron;
#[path = "cli_subcommands_provider.rs"]
mod cli_subcommands_provider;
#[path = "cli_subcommands_skill.rs"]
mod cli_subcommands_skill;
#[path = "cli_trajectory_cmds.rs"]
mod cli_trajectory_cmds;
#[path = "cli_tenant_cmds.rs"]
mod cli_tenant_cmds;
#[path = "cli_vertical_cmds.rs"]
mod cli_vertical_cmds;
#[path = "cli_workflow_cmds.rs"]
mod cli_workflow_cmds;
#[path = "cli_work_cmds.rs"]
pub(crate) mod cli_work_cmds;
#[path = "cli_worker_cmds.rs"]
pub(crate) mod cli_worker_cmds;
#[path = "cli_workspace_cmds.rs"]
pub(crate) mod cli_workspace_cmds;
pub use cli_action_cmds::*;
pub use cli_audit_cmds::AuditQueryCmd;
pub use cli_budget_cmds::*;
pub use cli_doctor_cmds::DoctorCmd;
pub use cli_genesis_cmds::*;
pub use cli_memory_cmd::SharedTeamCmd;
pub use cli_session_cmds::*;
pub use cli_subcommands::*;
pub use cli_tenant_cmds::TenantCmd;
pub use cli_work_cmds::WorkCmd;
pub use cli_worker_cmds::WorkerCmd;
pub use cli_workflow_cmds::WorkflowCmd;
pub use cli_trajectory_cmds::TrajectoryCmd;

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
    /// Start a fresh session row in the local store. Without a
    /// daemon to actually call the LLM this is just a header
    /// insert — useful for prototyping the read-side surface and
    /// for capturing externally-streamed messages via the gateway
    /// (`POST /sessions/<id>/messages`).
    New {
        /// Provider id (matches `manifests/providers/<id>.toml`).
        #[arg(long)]
        provider: String,
        /// Model id (e.g. claude-haiku, gpt-5-1).
        #[arg(long)]
        model: String,
        /// Optional human-readable title.
        #[arg(long)]
        title: Option<String>,
        /// Bind the new session to a vertical-template id (#651).
        /// The vertical must be discoverable via
        /// `argo vertical list`.
        #[arg(long)]
        vertical: Option<String>,
    },
    /// Re-render a recorded session: prints its header and every
    /// message in order. Read-only — for the live "continue this
    /// session against an LLM" flow, see the daemon's
    /// /sessions/<id>/runs binding (separate PR).
    Resume {
        /// Session id (full UUID).
        id: String,
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
    /// Work queue v0 (#716). `add` enqueues a unit of work for a
    /// vertical-bound worker; `serve` runs a persistent loop that
    /// processes the queue using existing session + memory + chat
    /// primitives. Default provider `mock` works without an API key.
    Work {
        #[command(subcommand)]
        cmd: WorkCmd,
    },
    /// Worker v3 execution-mode action ledger (#728). Inspect (and
    /// in later PRs: approve / execute / complete) the actions
    /// the worker has proposed. Distinct from `argo work` — this
    /// surface is the noun (the worker's output), not the verb
    /// (running the worker).
    Worker {
        #[command(subcommand)]
        cmd: WorkerCmd,
    },
    Tenant {
        #[command(subcommand)]
        cmd: TenantCmd,
    },
    /// Org chart, addressable names, manager chain
    /// (`claude-org-awareness`).
    Org {
        #[command(subcommand)]
        cmd: cli_org_cmds::OrgCmd,
    },
    /// Worker rosters: group-level policy, channels, budget cap
    /// (`claude-worker-roster`).
    Roster {
        #[command(subcommand)]
        cmd: cli_roster_cmds::RosterCmd,
    },
    /// Declarative org policies (Stream `claude-policy-as-code`):
    /// list, show, validate, apply YAML files at
    /// `<workspace>/policies/<name>.yaml` that constrain the
    /// auto-approval gate via the resolution chain
    /// action → worker → roster → workspace-file → default.
    Policy {
        #[command(subcommand)]
        cmd: cli_policy_cmds::PolicyCmd,
    },
    /// Declarative handoff routing rules
    /// (`claude-handoff-routing-rules`).
    HandoffRules {
        #[command(subcommand)]
        cmd: cli_handoff_cmds::HandoffRulesCmd,
    },
    /// Declarative DAG workflows across multiple workers (Stream
    /// `claude-workflow-dag`). YAML schema describing tasks +
    /// edges; runs each task as a Session with output binding
    /// from upstream tasks.
    Workflow {
        #[command(subcommand)]
        cmd: WorkflowCmd,
    },
    /// Export action trajectories for fine-tuning data pipelines
    /// (claude-trajectory-training-export).
    Trajectory {
        #[command(subcommand)]
        cmd: TrajectoryCmd,
    },
    /// Action debugging tools that do not mutate the action ledger.
    Action {
        #[command(subcommand)]
        cmd: ActionCmd,
    },
    Activity {
        #[command(subcommand)]
        cmd: ActivityCmd,
    },
    Audit {
        #[command(subcommand)]
        cmd: AuditQueryCmd,
    },
    /// First-time interactive setup. Mirrors the
    /// `curl … | bash && argo setup` Hermes-style two-step
    /// install (#646). Idempotent: re-runnable to verify
    /// configuration or pick a different provider.
    Setup {
        /// Verification mode — skip the interactive walkthrough
        /// and exit non-zero if no provider key is loadable from
        /// the process environment. Designed for CI / production-
        /// launch checklists; pairs with the
        /// `provider_keys.md` rotation runbook.
        #[arg(long)]
        check: bool,
        /// Run the setup wizard without prompts. Intended for CI
        /// and scripted bootstrap flows.
        #[arg(long)]
        non_interactive: bool,
        /// Emit `{steps, ready}` as JSON instead of the checklist table.
        #[arg(long)]
        json: bool,
        /// Require this specific env var to be set (repeatable).
        /// When provided alongside `--check`, the probe demands
        /// every named var resolves to a non-empty value;
        /// otherwise the existing "at least one" contract
        /// applies. Use this when your vertical declares both a
        /// primary AND a fallback provider — the launch
        /// checklist in `provider_keys.md` says BOTH should be
        /// loadable so the fallback actually works on 503.
        #[arg(long = "require", value_name = "ENV_VAR")]
        require: Vec<String>,
    },
    /// Vertical-template discovery surface (#650). Read-only —
    /// `list`, `show`, `validate`. Install / eval / run land in
    /// later PRs.
    Vertical {
        #[command(subcommand)]
        cmd: VerticalCmd,
    },
    /// Charter → vertical generator (#G-genesis). Operator
    /// describes a role in plain English; Genesis emits a
    /// complete `verticals/<id>/` tree with safety judge
    /// guarantees on every action.
    Genesis {
        #[command(subcommand)]
        cmd: GenesisCmd,
    },
    /// Workspace-wide message rollups. Mirrors the gateway's
    /// `/messages/*` family.
    Messages {
        #[command(subcommand)]
        cmd: MessagesCmd,
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
    Budget {
        #[command(subcommand)]
        cmd: BudgetCmd,
    },
    Session {
        #[command(subcommand)]
        cmd: SessionCmd,
    },
    Handoff {
        #[command(subcommand)]
        cmd: HandoffCmd,
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
    /// Validate eval JSONL, or run/list vertical eval fixtures.
    Eval {
        #[command(subcommand)]
        cmd: cli_eval_cmds::EvalCmd,
    },
    /// Run one turn of the agent loop against a recorded session:
    /// loads the session's messages, calls the configured provider
    /// (Anthropic / OpenAI / OpenAI Responses), and persists the
    /// assistant's response. Requires the provider's API-key env
    /// var.
    Chat {
        /// Session id (from `argo new` / `argo session list`).
        id: String,
        /// Optional fresh user message to append before the call.
        /// Equivalent to a `POST /sessions/<id>/messages` round-trip.
        #[arg(long)]
        user: Option<String>,
        /// Stream the response token-by-token to stdout. Final
        /// persisted message is the same — but you see it live.
        #[arg(long)]
        stream: bool,
        /// Drop into a REPL: read user input, run a turn, print
        /// the response, repeat. Ctrl-D / EOF exits cleanly.
        #[arg(long, short = 'i')]
        interactive: bool,
        /// Override `MAX_TOOL_ROUNDS` (default 8). Useful when
        /// debugging — set to 1 to fail fast on the first tool
        /// call, set higher for legitimately long planning chains.
        #[arg(long)]
        max_tool_rounds: Option<usize>,
    },
    /// Open the interactive TUI (read-only session viewer for now).
    Tui,
    /// List the in-core tools the agent can call. Mirrors
    /// `GET /tools`.
    Tools {
        #[command(subcommand)]
        cmd: Option<ToolsCmd>,
    },
    /// Diagnose configuration / sandbox / port issues.
    Doctor {
        #[command(subcommand)]
        cmd: Option<DoctorCmd>,
        /// Emit a single JSON object with all the checks. Default
        /// is the human-readable text report. Equivalent to
        /// `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Run every read-only verification — workspace, provider
    /// manifests, skill hashes, memory hashes, audit chain — and
    /// exit non-zero on the first failure. Suitable as a single
    /// CI hook.
    Health {
        /// Output a single JSON object with per-check status.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    Import {
        #[command(subcommand)]
        cmd: ImportCmd,
    },
    Update,
}

#[derive(Subcommand, Debug)]
pub enum ToolsCmd {
    /// List in-core tools. Default text view; `--json` emits
    /// one object per tool; `--format markdown` renders a
    /// markdown table.
    List {
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
        /// Case-insensitive substring filter on tool name.
        /// Mirrors `?name-contains=…` on `/tools`.
        #[arg(long)]
        name_contains: Option<String>,
    },
    /// Show one tool's full JSON-schema parameter shape by
    /// name. Mirrors `GET /tools/<name>`.
    Show {
        name: String,
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
}
