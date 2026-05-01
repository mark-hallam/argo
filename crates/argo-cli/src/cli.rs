//! clap-derived CLI surface (§19 command tree). The top-level
//! `Cli` + `Command` enums live here; per-family `Subcommand` enums
//! are extracted to `cli_subcommands.rs` (re-exported below) so
//! both files stay under the 500-LOC ceiling.
#[path = "cli_memory_cmd.rs"]
mod cli_memory_cmd;
#[path = "cli_subcommands.rs"]
mod cli_subcommands;
#[path = "cli_session_cmds.rs"]
mod cli_session_cmds;
#[path = "cli_workspace_cmds.rs"]
pub(crate) mod cli_workspace_cmds;
pub use cli_session_cmds::*;
pub use cli_subcommands::*;

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
