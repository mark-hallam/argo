use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum AgentsCmd {
    /// List `[agents.*]` blocks from `argo.toml`.
    List {
        /// Output as a JSON array for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Add a new `[agents.<name>]` block to `argo.toml`. Use the
    /// flags to populate keys; defaults are sensible enough that
    /// the agent shows up in `argo agents list` immediately.
    Create {
        name: String,
        /// `default_model` field. Defaults to "claude-3-5-sonnet".
        #[arg(long)]
        model: Option<String>,
        /// `default_toolset` field. Defaults to "default".
        #[arg(long)]
        toolset: Option<String>,
        /// `memory_namespace` field. Defaults to the agent name.
        #[arg(long)]
        memory_namespace: Option<String>,
    },
    /// Remove an `[agents.<name>]` block from `argo.toml`.
    Delete { name: String },
}

pub use super::cli_daemon_cmds::DaemonCmd;
pub use super::cli_duty_cmds::DutyCmd;
pub use super::cli_gateway_cmds::GatewayCmd;
pub use super::cli_vertical_cmds::VerticalCmd;

#[derive(Subcommand, Debug)]
pub enum McpCmd {
    /// Serve the workspace store as an MCP server over stdio.
    Serve,
    /// Discover the tools an MCP server exposes
    /// (`claude-mcp-tool-discovery-vis`). Connects to the server
    /// over stdio using the manifest declaration in the named
    /// vertical, calls `tools/list`, and renders the result.
    Tools {
        /// Server id as declared in the vertical manifest's
        /// `[mcp_servers]` section.
        server_id: String,
        /// Vertical id whose manifest declares the server.
        #[arg(long)]
        vertical: String,
        /// Output format: `markdown` (default) or `json`
        /// (NDJSON, one tool per line).
        #[arg(long, default_value = "markdown")]
        format: String,
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Regex narrowing the tool list by name.
        #[arg(long)]
        filter: Option<String>,
        /// Omit descriptions for terse output.
        #[arg(long)]
        schema_only: bool,
    },
    /// Manage the per-tenant MCP server registry
    /// (`codex-mcp-server-registry-cli`). Backs the dispatch path
    /// when `ARGO_MCP_REGISTRY_BACKED=1` is set.
    Registry {
        #[command(subcommand)]
        cmd: McpRegistryCmd,
    },
    /// Boot the `argo-mcp-image-gen` MCP server over stdio
    /// (`mission-image-generation-mcp-server`). Reads
    /// `[mcp.image_gen]` from `--config`, dispatches
    /// `image.generate` + `image.edit` to OpenAI-compatible or
    /// stub backends, writes a `cost_estimate` audit row before
    /// each call.
    ImageGen {
        /// Path to `argo.toml`. Defaults to `./argo.toml`.
        #[arg(long, default_value = "argo.toml")]
        config: String,
        /// Short-circuit calls with a cost estimate ≥ 20¢.
        #[arg(long)]
        require_confirm: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum McpRegistryCmd {
    /// List registered MCP servers for a tenant.
    List {
        /// Tenant id. Defaults to the current workspace tenant.
        #[arg(long)]
        tenant: Option<String>,
        /// Output format: `markdown` (default) or `json` (NDJSON).
        #[arg(long, default_value = "markdown")]
        format: String,
        /// Emit NDJSON; equivalent to `--format json`.
        #[arg(long)]
        json: bool,
    },
    /// Register one MCP server for a tenant.
    Add {
        /// Server id referenced by MCP-executor actions.
        server_id: String,
        /// Server kind or transport family (e.g. `stdio`, `http`).
        #[arg(long)]
        kind: String,
        /// Base URL for the MCP server.
        #[arg(long = "url")]
        base_url: String,
        /// Opaque reference into the workspace credential store.
        #[arg(long)]
        secrets_ref: Option<String>,
        /// Tenant id. Defaults to the current workspace tenant.
        #[arg(long)]
        tenant: Option<String>,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Remove one MCP server registration.
    Remove {
        /// Server id to remove.
        server_id: String,
        /// Tenant id. Defaults to the current workspace tenant.
        #[arg(long)]
        tenant: Option<String>,
        /// Remove even when active actions reference this server.
        #[arg(long)]
        force: bool,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum AcpCmd {
    Serve,
}

#[derive(Subcommand, Debug)]
pub enum PluginCmd {
    /// List plugins configured under `[plugin].auto_load`.
    List {
        /// Output as a JSON array of strings for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    Install { source: String },
    Enable { name: String },
    Disable { name: String },
    Remove { name: String },
    Run { name: String },
}

pub use super::cli_subcommands_provider::{ModelCmd, ProviderCmd};
pub use super::cli_subcommands_skill::{
    SkillCmd, SkillPublishCmd, SkillTagsCmd, SkillUnpublishCmd,
};
pub use super::cli_memory_cmd::MemoryCmd;
pub use super::cli_cred_cmds::CredCmd;
pub use super::cli_backup_cmds::{BackupCmd, RestoreCmd};
#[path = "cli_connector_cmds.rs"]
mod cli_connector_cmds;
pub use cli_connector_cmds::ConnectorCmd;
pub use super::cli_subcommands_cron::CronCmd;

#[derive(Subcommand, Debug)]
pub enum AuditCmd {
    /// Show the kind histogram + (optionally) the last `--tail N`
    /// rows. Pass `--session <id>` to print every audit row for one
    /// session in `seq` order instead of the global histogram view.
    Show {
        #[arg(long)]
        tail: Option<u32>,
        /// Output as a single JSON object (kind histogram + tail).
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
        /// Session id to scope to. With this set, the output is the
        /// session's full audit chain (seq order, oldest first).
        #[arg(long)]
        session: Option<String>,
        /// Comma-separated list of audit kinds to include (case-
        /// sensitive label match, e.g. `ToolCall,ToolResult`). When
        /// set, the tail / per-session view drops every other kind.
        #[arg(long)]
        kinds: Option<String>,
        /// Filter to rows created within this many recent minutes.
        /// Applies to both the tail view and `--session`.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// `tail -f` style: poll for new rows every 250ms and
        /// print them as they arrive. Combine with `--session <id>`
        /// to watch one session live; Ctrl-C to exit.
        #[arg(long)]
        watch: bool,
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
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
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
    /// Print just the chain-break rows (`{seq, kind, reason}`).
    /// Mirrors `GET /audit/breaks` (#406). One row per line in the
    /// default view, or a single JSON array with `--json`.
    Breaks {
        /// Output as a single JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Print just the count of audit rows after filters.
    /// Mirrors `GET /audit/count` (#458).
    Count {
        /// Filter by audit kind (e.g. tool_call, turn_start).
        #[arg(long)]
        kind: Option<String>,
        /// Filter to rows created within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Filter to rows on one session (uuid).
        #[arg(long)]
        session: Option<String>,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Histogram of audit rows grouped by `kind`. Mirrors the
    /// gateway's `GET /audit/by-kind` (#570).
    ByKind {
        /// Filter to rows created within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object (`{by_kind}`).
        #[arg(long)]
        json: bool,
    },
    /// Per-vertical audit-row histogram (#688). Joins
    /// audit_record.session_id → session.vertical_id; rows from
    /// unbound sessions / no-session-id rows fall in `(none)`.
    ByVertical {
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    /// Daily audit-row rollup. Mirrors the gateway's
    /// `GET /audit/by-day` (#571).
    ByDay {
        /// Lookback window in days (default 30, cap 365).
        #[arg(long, default_value_t = 30)]
        days: u32,
        /// Output as a single JSON object (`{by_day}`).
        #[arg(long)]
        json: bool,
    },
    /// Top-N sessions by audit row count. Mirrors the gateway's
    /// `GET /audit/by-session` (#608).
    BySession {
        /// Top-N session limit (default 50, cap 500).
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Filter to rows created within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object (`{by_session}`).
        #[arg(long)]
        json: bool,
    },
    /// Histogram of `tool_call`+`tool_result` audit rows grouped by
    /// tool name. Mirrors the gateway's `GET /audit/by-tool` (#613).
    ByTool {
        /// Filter to rows created within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object (`{by_tool}`).
        #[arg(long)]
        json: bool,
    },
    /// Histogram of `tool_result` audit rows where `is_error=true`,
    /// grouped by tool. Mirrors the gateway's
    /// `GET /audit/tool-errors` (#617). Operational signal for
    /// spotting broken tool integrations.
    ToolErrors {
        /// Filter to rows created within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object (`{tool_errors}`).
        #[arg(long)]
        json: bool,
    },
    /// 2D rollup: audit rows bucketed by `(day, kind)`. Mirrors
    /// the gateway's `GET /audit/by-day-by-kind` (#638). Useful
    /// for "did tool_call volume spike Tuesday?" without joining
    /// by-day + by-kind client-side.
    ByDayByKind {
        /// Lookback window in days (default 30, cap 365).
        #[arg(long, default_value_t = 30)]
        days: u32,
        /// Output as a single JSON object (`{by_day_by_kind}`).
        #[arg(long)]
        json: bool,
    },
    /// Replay one session's audit chain in seq order for incident
    /// investigation (Stream `claude-audit-replay`). `pretty`
    /// (default) prints one-line headlines per row; `raw` emits
    /// NDJSON for piping into `jq`.
    Replay {
        /// Session id (UUID) to replay.
        session_id: String,
        /// Lower bound on `seq` (inclusive). Skip every row before
        /// this — useful when only the tail of a long chain is
        /// relevant.
        #[arg(long)]
        from_seq: Option<i64>,
        /// Upper bound (inclusive) on `created_at` as RFC-3339
        /// (e.g. `2026-05-07T10:00:00Z`). Filters out anything
        /// after this instant.
        #[arg(long)]
        until: Option<String>,
        /// Output format: `pretty` (default) or `raw` (NDJSON).
        #[arg(long, default_value = "pretty")]
        format: String,
    },
}

pub use super::cli_workspace_cmds::{CostsCmd, DbCmd, WorkspaceCmd};

pub use super::cli_activity_cmds::ActivityCmd;
pub use super::cli_classify_cmds::ClassifyCmd;
pub use super::cli_plan_cmds::PlanCmd;
pub use super::cli_skill_conflicts_cmds::SkillConflictsCmd;
pub use super::cli_skill_review_cmds::SkillReviewCmd;
pub use super::cli_subtask_cmds::SubtaskCmd;
