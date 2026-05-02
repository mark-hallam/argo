//! Per-family clap `Subcommand` enums. Hoisted out of `cli.rs`
//! to keep both files under the 500-LOC ceiling. Re-exported
//! wholesale by `cli.rs` so handler imports (`use crate::cli::FooCmd;`)
//! continue to work unchanged.
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

// `GatewayCmd` was extracted to cli_gateway_cmds.rs (#631)
// when this file went over the 500-LOC budget. Re-exported by
// the wildcard glob in cli.rs so callers don't need to know
// the new path.
pub use super::cli_gateway_cmds::GatewayCmd;
// Same pattern for VerticalCmd (#650).
pub use super::cli_vertical_cmds::VerticalCmd;

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

#[derive(Subcommand, Debug)]
pub enum ProviderCmd {
    /// List provider manifests under `manifests/providers/`.
    List {
        /// Output as a JSON array of `{id, family, base_url}` rows.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
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
    List {
        /// Output as JSON. Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    Set { model: String },
    Test { model: String },
}

#[derive(Subcommand, Debug)]
pub enum SkillCmd {
    List {
        /// Output as JSON (one object per skill) for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`
        /// (renders a markdown table). `--json` wins if both are
        /// passed.
        #[arg(long, default_value = "text")]
        format: String,
        /// Sort order: `created-at` (default, descending) or
        /// `name` (lexical, ascending).
        #[arg(long, default_value = "created-at")]
        sort_by: String,
        /// Truncate to the first N rows (after sorting).
        #[arg(long)]
        limit: Option<u32>,
        /// Include superseded versions (default keeps only the
        /// active skill per name). Mirrors `?include-superseded`.
        #[arg(long)]
        include_superseded: bool,
        /// Case-insensitive substring filter on `frontmatter.name`.
        /// Mirrors `?name-contains=…`.
        #[arg(long)]
        name_contains: Option<String>,
        /// Filter by lock status: `none` / `user-protected` /
        /// `content-locked`. Mirrors `?lock=…`.
        #[arg(long)]
        lock: Option<String>,
        /// Case-insensitive substring filter on the skill body.
        /// Mirrors `?body-contains=…`.
        #[arg(long)]
        body_contains: Option<String>,
    },
    /// Print just the count of active skills. Default: one int
    /// per line; `--json` emits `{count}`.
    Count {
        /// Include superseded versions in the count too.
        #[arg(long)]
        include_superseded: bool,
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
    /// Create a new skill from CLI flags or stdin. Inserts a
    /// fresh row via `SkillStore::install`; rejects duplicate
    /// names (use `argo skill refine` to bump an existing
    /// skill instead).
    Create {
        name: String,
        /// One-line description for the frontmatter. Defaults
        /// to a stub (`<name> — created via argo skill create`).
        #[arg(long)]
        description: Option<String>,
        /// Skill body text. Mutually exclusive with
        /// `--from-file`; without either flag, body is read
        /// from stdin until EOF.
        #[arg(long, conflicts_with = "from_file")]
        body: Option<String>,
        /// Read the skill body from a path on disk.
        #[arg(long)]
        from_file: Option<String>,
        /// Version label for the new row. Defaults to "0.1".
        #[arg(long = "new-version", default_value = "0.1")]
        new_version: String,
    },
    /// Append a hint to the active skill's body and supersede it
    /// with a fresh version. Manual mutator — the score-driven
    /// version (run bench, accept only when the new score wins)
    /// lands once `forge optimize` learns to mutate.
    Refine {
        name: String,
        /// Text to append to the skill body, under a new
        /// `## Refinement` section.
        #[arg(long)]
        hint: String,
        /// Version label for the refined skill. Defaults to
        /// bumping the existing version's last numeric segment
        /// (e.g. `0.1` → `0.2`). `--version` would clash with
        /// clap's auto-version flag, hence the rename.
        #[arg(long = "new-version")]
        new_version: Option<String>,
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

// MemoryCmd moved to `cli_memory_cmd.rs` to keep this file
// under the 500-LOC budget. Re-exported below.
pub use super::cli_memory_cmd::MemoryCmd;

#[derive(Subcommand, Debug)]
pub enum CronCmd {
    /// List scheduled jobs in the workspace's `cron/jobs/` directory.
    List {
        /// Output as JSON (one object per job) for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
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
}

// WorkspaceCmd / DbCmd / CostsCmd live in
// `super::cli_workspace_cmds` to keep this file under the
// 500-LOC budget. Re-exported via `cli.rs` so callers don't
// need to know the new path.
pub use super::cli_workspace_cmds::{CostsCmd, DbCmd, WorkspaceCmd};

