//! Top-level direct-action operator commands extracted from
//! `cli_command_enum.rs` so the entrypoint stays under the 500-LOC
//! ceiling. Cohesive group: session lifecycle (`new` / `resume` /
//! `chat` / `steer` / `trace` / `recover`), runtime diagnostics
//! (`doctor` / `health` / `latency` / `voice`), and system-level ops
//! (`setup` / `onboard` / `tui` / `tools` / `update`). Wired into the
//! parent `Command` enum via `#[command(flatten)]`, so the
//! `argo --help` subcommand list is identical pre/post split.

use super::*;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum RuntimeCmd {
    /// Start a fresh session row in the local store. Without a
    /// daemon to actually call the LLM this is just a header
    /// insert — useful for prototyping the read-side surface and
    /// for capturing externally-streamed messages via the gateway
    /// (`POST /sessions/<id>/messages`).
    #[command(after_help = "Examples:\n  \
        argo new --provider anthropic --model claude-sonnet-4-6\n  \
        argo new --provider mock --model mock --title \"smoke test\"\n  \
        argo new --provider anthropic --model claude-sonnet-4-6 --vertical gtm-engineer")]
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
    #[command(after_help = "Examples:\n  \
        argo resume 01J9XYZ...")]
    Resume {
        /// Session id (full UUID).
        id: String,
    },
    /// First-time interactive setup. Mirrors the
    /// `curl … | bash && argo setup` Hermes-style two-step
    /// install (#646). Idempotent: re-runnable to verify
    /// configuration or pick a different provider.
    #[command(after_help = "Examples:\n  \
        argo setup\n  \
        argo setup --check\n  \
        argo setup --check --require ANTHROPIC_API_KEY --require OPENAI_API_KEY")]
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
    #[command(after_help = "Examples:\n  \
        argo onboard\n  \
        argo onboard --non-interactive\n  \
        argo onboard --non-interactive --json --config .argo/onboarding.toml")]
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
    /// Append an out-of-band user message to a live session. Used to
    /// nudge a running agent without waiting for the next prompt.
    #[command(after_help = "Examples:\n  \
        argo steer 01J9XYZ... \"pause and summarise progress\"")]
    Steer {
        session_id: String,
        text: String,
    },
    /// Inspect session traces. Without a subcommand, renders the
    /// audit-log timeline (`--session`) or one turn's span tree
    /// (`--turn`); with `explore <session>`, walks the local OTLP
    /// NDJSON sink and renders a markdown or JSON tree.
    #[command(after_help = "Examples:\n  \
        argo trace --session 01J9XYZ...\n  \
        argo trace --turn 01J9TURN...\n  \
        argo trace explore 01J9XYZ... --format json")]
    Trace {
        #[command(subcommand)]
        cmd: Option<TraceCmd>,
        #[arg(long)]
        turn: Option<String>,
        #[arg(long)]
        session: Option<String>,
    },
    /// Run one turn of the agent loop against a recorded session:
    /// loads the session's messages, calls the configured provider
    /// (Anthropic / OpenAI / OpenAI Responses), and persists the
    /// assistant's response. Requires the provider's API-key env
    /// var.
    #[command(after_help = "Examples:\n  \
        export ANTHROPIC_API_KEY=sk-ant-...\n  \
        argo chat 01J9XYZ... --user \"summarise progress so far\"\n  \
        argo chat 01J9XYZ... --interactive --stream")]
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
    #[command(after_help = "Examples:\n  \
        argo tui")]
    Tui,
    /// List the in-core tools the agent can call. Mirrors
    /// `GET /tools`.
    #[command(after_help = "Examples:\n  \
        argo tools\n  \
        argo tools list --format markdown\n  \
        argo tools show web_search --json")]
    Tools {
        #[command(subcommand)]
        cmd: Option<ToolsCmd>,
    },
    /// Diagnose configuration / sandbox / port issues.
    #[command(after_help = "Examples:\n  \
        argo doctor\n  \
        argo doctor --json")]
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
    #[command(after_help = "Examples:\n  \
        argo health\n  \
        argo health --json")]
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
    /// LLM-assisted post-mortem for a failed session (Stream
    /// `claude-error-recovery-suggestions`). Walks the audit
    /// chain, calls Anthropic with the chain summary, returns
    /// 3 ranked fix suggestions. Cached per
    /// (session_id, audit_chain_hash) so re-runs are free.
    /// Advisory only — no auto-apply.
    #[command(after_help = "Examples:\n  \
        argo recover 01J9XYZ...\n  \
        argo recover 01J9XYZ... --json")]
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
    #[command(after_help = "Examples:\n  \
        argo voice\n  \
        argo voice --mock-script fixtures/voice-mock.json --json")]
    Voice {
        /// Drive the loop from a JSON file (`{"script": [...]}`).
        #[arg(long, value_name = "PATH")]
        mock_script: Option<String>,
        /// Emit one TickOutcome per line as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show per-executor-kind action latency percentiles (Stream
    /// `claude-latency-percentiles-tracking`). Reads the
    /// `latency_buckets` table populated by the dispatch wrapper
    /// and prints `kind | n | p50 | p95 | p99 | max` as a markdown
    /// table (default) or as JSON.
    #[command(after_help = "Examples:\n  \
        argo latency\n  \
        argo latency --format json")]
    Latency {
        /// Output format: `markdown` (default) or `json`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Placeholder for an in-place upgrade command. Not yet wired
    /// in this build — see `argo self-update` for the working
    /// signed-release upgrade flow.
    #[command(after_help = "Examples:\n  \
        argo update                # stub today, exits non-zero\n  \
        argo self-update --check   # use this instead")]
    Update,
}
