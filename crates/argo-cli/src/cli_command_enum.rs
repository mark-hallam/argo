//! Top-level `Command` enum split out of `cli.rs` so the entrypoint stays under the LOC ceiling.

use super::*;
use clap::Subcommand;

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
    /// Workspace credential storage and shell export helpers.
    Cred {
        #[command(subcommand)]
        cmd: CredCmd,
    },
    /// Workspace backup create/list/verify.
    Backup {
        #[command(subcommand)]
        cmd: BackupCmd,
    },
    /// Restore a workspace backup tarball.
    Restore {
        #[command(flatten)]
        args: RestoreCmd,
    },
    /// macOS launchd-agent install/uninstall (Stream
    /// `claude-installer-macos`). Writes
    /// `~/Library/LaunchAgents/com.skycot.argo.plist` so the
    /// gateway starts on login + auto-restarts on crash.
    /// macOS-only.
    Daemon {
        #[command(subcommand)]
        cmd: DaemonCmd,
    },
    /// Manage OAuth and token credentials for MCP connectors.
    Connector {
        #[command(subcommand)]
        cmd: ConnectorCmd,
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
    /// Persona maintenance — `$EDITOR`-driven update of the
    /// human-facing persona row attached to one or more workers
    /// (Stream `claude-persona-edit-cli`).
    Persona {
        #[command(subcommand)]
        cmd: PersonaCmd,
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
    /// Worker duty scheduler — autonomous cron-driven actions
    /// (Stream `claude-worker-duty-scheduler`, V1.3). list / show /
    /// pause / resume / fire-now.
    Duty {
        #[command(subcommand)]
        cmd: DutyCmd,
    },
    /// Inspect handoff fallback auctions
    /// (`claude-skill-auction`). When the routing chain
    /// exhausts, the walker may record a `no_bidders` auction
    /// row in the audit trail; this surface lets operators read
    /// + cancel those.
    HandoffAuctions {
        #[command(subcommand)]
        cmd: cli_handoff_cmds::HandoffAuctionsCmd,
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
    /// First-experience operator walkthrough — workspace bootstrap,
    /// credentials, first connector, first worker (genesis), first
    /// action, done. 7 stages; stages 3-7 may Skip cleanly. The
    /// non-developer's entry point — pairs with `argo setup` (the
    /// CI-style provider-key probe).
    Onboard {
        /// Run without prompts. Reads `<workspace>/.argo/onboarding.toml`
        /// (or `--config <path>`) for stage inputs.
        #[arg(long)]
        non_interactive: bool,
        /// Emit `{stages, any_fail, count}` as JSON instead of the
        /// pretty checklist.
        #[arg(long)]
        json: bool,
        /// Override the default `<workspace>/.argo/onboarding.toml`
        /// path used by `--non-interactive`.
        #[arg(long, value_name = "PATH")]
        config: Option<String>,
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
    /// Operator-supplied thumbs up/down per action (Stream
    /// `claude-operator-feedback-channel`). Feeds the
    /// skill-learning loop as a strong-positive / strong-negative
    /// signal alongside the heuristic success metric.
    Feedback {
        #[command(subcommand)]
        cmd: FeedbackCmd,
    },
    /// Tag records (memory rows, audit payloads, action params)
    /// with data classes (Stream `claude-data-classification`):
    /// `pii` / `phi` / `financial` / `secret` / `public` /
    /// `untagged`. Drives per-class retention windows and
    /// audit-export redaction.
    Classify {
        #[command(subcommand)]
        cmd: ClassifyCmd,
    },
    /// Decompose a coarse goal into a workflow DAG (Stream
    /// `claude-runtime-planner-trait`). Offline TemplatePlanner by
    /// default; `--llm` opts into the Anthropic-backed LlmPlanner.
    Plan {
        #[command(subcommand)]
        cmd: PlanCmd,
    },
    /// Inspect the persistent subtask ledger
    /// (Stream `claude-subtask-ledger-store`). One row per node the
    /// Planner emits during decomposition; durable across restarts
    /// so operators can read a worker's plan after the fact.
    Subtask {
        #[command(subcommand)]
        cmd: SubtaskCmd,
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
    /// LLM-assisted post-mortem for a failed session (Stream
    /// `claude-error-recovery-suggestions`). Walks the audit
    /// chain, calls Anthropic with the chain summary, returns
    /// 3 ranked fix suggestions. Cached per
    /// (session_id, audit_chain_hash) so re-runs are free.
    /// Advisory only — no auto-apply.
    Recover {
        /// Session id (UUID) whose chain to analyze.
        session_id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Inspect or invalidate cached suggestions written by `argo
    /// recover` (Stream `codex-recover-cache-cli`). Pure CRUD over
    /// the `recovery_cache` table — no LLM calls.
    RecoverCache {
        #[command(subcommand)]
        cmd: cli_recover_cmds::RecoverCacheCmd,
    },
    /// Local voice TUI for approval flow
    /// (`claude-voice-local-tui-whisper`). Mic capture is
    /// opt-in: `--features whisper-native`.
    Voice {
        /// Drive the loop from a JSON file (`{"script": [...]}`).
        #[arg(long, value_name = "PATH")]
        mock_script: Option<String>,
        /// Emit one TickOutcome per line as JSON.
        #[arg(long)]
        json: bool,
    },
    Update,
}
