//! Per-family clap `Subcommand` enums. Hoisted out of `cli.rs`
//! to keep both files under the 500-LOC ceiling. Re-exported
//! wholesale by `cli.rs` so handler imports (`use crate::cli::FooCmd;`)
//! continue to work unchanged.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum AgentsCmd {
    /// List `[agents.*]` blocks from `argo.toml`.
    List,
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

#[derive(Subcommand, Debug)]
pub enum GatewayCmd {
    /// Bind a TCP listener and serve the JSON HTTP gateway. The
    /// first cut answers `GET /health` + `GET /version`; the
    /// `Adapter` routing surface from `argo-gateway` lands later.
    Start {
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        #[arg(long, default_value_t = 7400)]
        port: u16,
        /// Stop after N requests (mainly for tests; omit to run
        /// until SIGINT / SIGTERM).
        #[arg(long)]
        max_requests: Option<u64>,
    },
    Stop,
    Status,
    /// Print the gateway's static route catalog. No daemon
    /// required — reads the in-binary catalog. `--method
    /// GET|POST` filters by verb; `--grep <sub>` filters by
    /// path/description substring (case-insensitive); `--limit
    /// N` truncates to the first N rows; `--json` for the
    /// machine-readable form.
    Routes {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        method: Option<String>,
        /// Case-insensitive substring filter on both path and
        /// description.
        #[arg(long)]
        grep: Option<String>,
        /// Truncate the output to the first N rows after filters.
        #[arg(long)]
        limit: Option<usize>,
    },
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
    List,
    Set { model: String },
    Test { model: String },
}

#[derive(Subcommand, Debug)]
pub enum SkillCmd {
    List {
        /// Output as JSON (one object per skill) for scripting.
        #[arg(long)]
        json: bool,
        /// Sort order: `created-at` (default, descending) or
        /// `name` (lexical, ascending).
        #[arg(long, default_value = "created-at")]
        sort_by: String,
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

#[derive(Subcommand, Debug)]
pub enum MemoryCmd {
    List {
        /// Output as JSON (one object per entry) for scripting.
        #[arg(long)]
        json: bool,
        /// Limit to one file: memory|user|soul|agents.
        #[arg(long)]
        file: Option<String>,
        /// Sort order: `learned-at` (default, descending) or
        /// `size` (body byte length descending — biggest first).
        #[arg(long, default_value = "learned-at")]
        sort_by: String,
    },
    View {
        entry: String,
        /// Output the entry as a single JSON object (incl. body).
        #[arg(long)]
        json: bool,
    },
    /// Append a fresh memory entry. `<file>` is one of memory|user|soul|agents.
    Append {
        file: String,
        body: String,
    },
    /// Print just the count of active memory entries. Mirrors
    /// `argo session count`. Default: one int per line; `--json`
    /// emits `{count, file}`.
    Count {
        /// Limit to one file: memory|user|soul|agents (default
        /// counts across all four).
        #[arg(long)]
        file: Option<String>,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Verify a memory entry's body matches its stored SHA-256 hash.
    /// Pass `--all` to verify every active entry across all four
    /// memory files in the workspace.
    Verify {
        #[arg(required_unless_present = "all")]
        entry: Option<String>,
        /// Verify every active memory entry instead of one named entry.
        #[arg(long, conflicts_with = "entry")]
        all: bool,
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
    List {
        /// Output as JSON (one object per job) for scripting.
        #[arg(long)]
        json: bool,
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
        #[arg(long)]
        json: bool,
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
        #[arg(long)]
        json: bool,
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
}

#[derive(Subcommand, Debug)]
pub enum WorkspaceCmd {
    /// Initialize a fresh workspace at `.` (writes argo.toml + the
    /// canonical bundle directories).
    Init,
    /// One-line status summary (workspace + db + costs).
    Status {
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Verify workspace integrity: argo.toml parses, memory files +
    ///   bundle dirs exist, agent persona files exist, manifests
    ///   referenced from `[provider.profiles.*]` resolve.
    Verify,
    /// Pull a workspace bundle from an OCI ref and extract it
    /// into `--into <dir>` (defaults to a sibling directory named
    /// after the repo). The destination directory must not exist.
    Fork {
        /// `<host>/<repo>:<tag>` reference, e.g.
        /// `ghcr.io/argo-agents/example:v1`.
        source: String,
        /// Destination directory. Defaults to
        /// `./fork-<repo>-<tag>` next to the cwd.
        #[arg(long)]
        into: Option<String>,
    },
    Push {
        oci_ref: String,
    },
    Pull {
        oci_ref: String,
    },
    /// List recorded marketplace pushes (newest first). Each row
    /// shows the OCI reference, bundle digest, pushed-at
    /// timestamp, and `pushed_by` attribution.
    Artifacts {
        #[arg(long, default_value_t = 25)]
        limit: u32,
        /// Output as JSON (one object per row) for scripting.
        #[arg(long)]
        json: bool,
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
    /// Pass `--reset` instead of a `<usd>` value to remove the
    /// cap (the field falls back to the workspace default).
    Cap {
        #[arg(long, conflicts_with_all = ["day", "month"])]
        session: bool,
        #[arg(long, conflicts_with_all = ["session", "month"])]
        day: bool,
        #[arg(long, conflicts_with_all = ["session", "day"])]
        month: bool,
        /// USD cap value. Either this OR `--reset` must be set.
        #[arg(conflicts_with = "reset")]
        usd: Option<f64>,
        /// Remove the cap field instead of setting it.
        #[arg(long)]
        reset: bool,
    },
    /// Show the configured caps + actual-spend roll-up.
    Show {
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
        /// Group spend by `(provider, model)` and print the
        /// top spenders instead of the default ceilings view.
        #[arg(long, conflicts_with = "by_session")]
        by_model: bool,
        /// Group spend by `session_id` and print the top
        /// spenders instead of the default ceilings view.
        #[arg(long)]
        by_session: bool,
    },
    /// Dump every row in `spend_ledger` as NDJSON (one JSON object
    /// per line). Pass `--out <path>` to write to a file; otherwise
    /// streams to stdout. `--since-minutes` scopes to recent rows.
    Export {
        #[arg(long)]
        out: Option<String>,
        /// Filter to rows charged within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
    },
    /// List sessions whose total spend exceeds `--threshold-usd`.
    /// Useful for spotting runaway sessions / abuse / model
    /// misconfiguration without parsing the ledger by hand.
    Alert {
        /// USD threshold; sessions with `SUM(amount_usd_micros)` >
        /// `threshold * 1e6` are reported.
        #[arg(long, default_value_t = 1.0)]
        threshold_usd: f64,
        /// Output as NDJSON (one row per offending session).
        #[arg(long)]
        json: bool,
    },
    /// Print just the today / month-to-date / all-time spend
    /// roll-up, nothing else. Mirrors the gateway's
    /// `GET /costs/total` endpoint.
    Total {
        /// Output as a single JSON object (`{today, this_month,
        /// all_time}` in usd_micros) for scripting.
        #[arg(long)]
        json: bool,
    },
}

