//! Top-level `Command` enum split out of `cli.rs` so the entrypoint stays under the LOC ceiling.
//!
//! Direct-action operator commands (session lifecycle, runtime
//! diagnostics, system ops) live in the sibling
//! `cli_runtime_cmds::RuntimeCmd` enum and are folded back in here
//! via `#[command(flatten)]` — so the `argo --help` subcommand list
//! is identical pre/post split.

use super::*;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum Command {
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
    /// Approval audit surfaces.
    Approval {
        #[command(subcommand)]
        cmd: ApprovalCmd,
    },
    /// Compliance surfaces (Phase C). Read-only over `audit_record`
    /// and tenant-scoped tables.
    /// - `soc2` — SOC2 Common Criteria 6.x audit-trail report
    ///   (`claude-compliance-soc2-audit-trail`).
    /// - `export` — GDPR Article 15 right-to-access JSON envelope
    ///   for one tenant; the export is itself audit-logged
    ///   (`claude-compliance-gdpr-data-export`).
    Compliance {
        #[command(subcommand)]
        cmd: ComplianceCmd,
    },
    Tenant {
        #[command(subcommand)]
        cmd: TenantCmd,
    },
    /// Operator-scoped surfaces. Currently:
    /// - `risk-overrides` — per-operator overrides of the
    ///   manifest-driven default risk classification (Stream
    ///   `mission-operator-specific-risk-overrides`). Upgrade-only
    ///   by default; downgrades require `--allow-downgrade` so a
    ///   compromised account can't silently relax approvals.
    Operator {
        #[command(subcommand)]
        cmd: OperatorCmd,
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
    /// Org-chart-aware teams: the operator-facing bridge between
    /// `argo org` (the tree) and `argo roster` (group policy)
    /// (Stream `claude-worker-team-roster-cli`). Each `team` is a
    /// single id that resolves uniquely in both `org_unit` (name +
    /// parent link) and `roster` (default policy bundle).
    Team {
        #[command(subcommand)]
        cmd: cli_team_cmds::TeamCmd,
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
    /// Workspace-level handoff policy
    /// (`claude-worker-handoff-policy`). Reads
    /// `.skycot/handoff-policy.toml` and answers "should we hand
    /// off at all?" via a first-match-wins rule list.
    HandoffPolicy {
        #[command(subcommand)]
        cmd: cli_handoff_cmds::HandoffPolicyCmd,
    },
    /// Aggregate metrics over the orchestrator's `handoff` table
    /// (Stream `codex-handoff-stats-cli`). Markdown report by
    /// default; `--histogram` switches to an ASCII volume readout.
    HandoffStats {
        #[command(flatten)]
        args: cli_handoff_cmds::HandoffStatsArgs,
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
    /// Validate eval JSONL, or run/list vertical eval fixtures.
    Eval {
        #[command(subcommand)]
        cmd: cli_eval_cmds::EvalCmd,
    },
    /// Walk the operator through a blameless postmortem (Stream
    /// `claude-incident-postmortem-template`). Reads stdin section
    /// by section, or `--from <yaml>` for non-interactive scripts;
    /// writes `docs/skycot/incidents/<YYYY-MM-DD>-<slug>.md`.
    Incident {
        #[command(subcommand)]
        cmd: IncidentCmd,
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
    Import {
        #[command(subcommand)]
        cmd: ImportCmd,
    },
    /// Computer-use trajectory inspection
    /// (`claude-cu-action-replay-cli`). `replay <session-id>`
    /// renders a recorded CU dispatch as markdown (+ optional GIF).
    Cu {
        #[command(subcommand)]
        cmd: cli_cu_cmds::CuCmd,
    },
    /// Apply / validate / show a `<role>.contract.toml` delegation
    /// contract (Stream `claude-delegation-contract-template`,
    /// V1.1 mission claim 2). Single artefact that captures
    /// charter + duties + KPI definitions + objectives + KRs +
    /// success criteria; `apply` materialises it into an existing
    /// worker through the same identity / OKR primitives that
    /// `argo worker charter set` and `argo worker okr set` already
    /// use.
    Contract {
        #[command(subcommand)]
        cmd: cli_contract_cmds::ContractCmd,
    },
    /// Cross-tenant trust-graph queries (Stream
    /// `mission-cross-tenant-trust-graph`, MISSION_BACKLOG Section 3
    /// Claim 4 #81). `argo trust graph --from <t>` prints outgoing
    /// edges; `argo trust composed --from <a> --to <c>` prints the
    /// composed score and BFS path.
    Trust {
        #[command(subcommand)]
        cmd: cli_trust_cmds::TrustCmd,
    },
    /// Verified binary self-upgrade (Stream `codex-self-update-cli`,
    /// V1.4). Queries GitHub Releases for a newer build, downloads
    /// the platform asset + detached Ed25519 signature, verifies
    /// against the embedded release pubkey, and atomically replaces
    /// the running binary. Best-effort gateway daemon restart.
    SelfUpdate {
        /// Report the latest available version without installing.
        #[arg(long)]
        check: bool,
        /// Release channel: `stable` (default) or `beta` (allows
        /// pre-release tags).
        #[arg(long, default_value = "stable")]
        channel: String,
        /// Skip the "already on latest" short-circuit and reinstall.
        /// Never bypasses signature verification.
        #[arg(long)]
        force: bool,
        /// Emit a JSON envelope instead of human-readable status.
        #[arg(long)]
        json: bool,
    },
    /// Direct-action operator commands (session lifecycle, runtime
    /// diagnostics, system ops). Flattened into the top-level
    /// `--help` subcommand list — `argo new`, `argo chat`,
    /// `argo doctor`, etc. all dispatch through this wrapper.
    #[command(flatten)]
    Runtime(cli_runtime_cmds::RuntimeCmd),
    /// End-to-end operator setup wizard (Mission
    /// `mission-operator-onboarding-wizard`). `argo wizard onboard`
    /// replaces the five-command first-time bootstrap (detect →
    /// provider key → operator identity → vertical → first worker)
    /// with a single guided flow.
    Wizard {
        #[command(subcommand)]
        cmd: cli_wizard_cmds::WizardCmd,
    },
}
