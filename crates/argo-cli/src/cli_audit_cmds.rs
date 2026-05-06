use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum AuditQueryCmd {
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
    /// Query audit rows with H-stack filters. Filters compose with
    /// each other and with `--since`, `--until`, and `--limit`.
    Query {
        /// Filter auto-approval rows by decision.
        #[arg(long)]
        decision: Option<String>,
        /// Filter rows by payload `worker_id`.
        #[arg(long)]
        worker: Option<String>,
        /// Filter rows by session UUID.
        #[arg(long)]
        session: Option<String>,
        /// Include escalation/handoff lifecycle audit rows.
        #[arg(long)]
        escalation: bool,
        /// Lower created-at bound, RFC3339/ISO-8601 timestamp.
        #[arg(long)]
        since: Option<String>,
        /// Upper created-at bound, RFC3339/ISO-8601 timestamp.
        #[arg(long)]
        until: Option<String>,
        /// Maximum rows to return.
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Output as a single JSON object with query envelope.
        #[arg(long)]
        json: bool,
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
    /// Export audit rows for compliance review.
    Export {
        /// Output format: ndjson, csv, or tarball.
        #[arg(long, default_value = "ndjson")]
        format: String,
        /// Write output to a path instead of stdout.
        #[arg(long = "bundle-out", alias = "out")]
        bundle_out: Option<String>,
        /// Lower created-at bound: YYYY-MM-DD or RFC3339.
        #[arg(long)]
        since: Option<String>,
        /// Upper created-at bound: YYYY-MM-DD or RFC3339.
        #[arg(long)]
        until: Option<String>,
        /// Filter by payload worker id.
        #[arg(long)]
        worker_id: Option<String>,
        /// Filter by tenant id.
        #[arg(long)]
        tenant: Option<String>,
        /// Legacy session filter retained for incident drill-downs.
        #[arg(long)]
        session: Option<String>,
        /// Filter to rows created within this many recent minutes.
        /// Combined with `--session` if both are set.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Replace emails in payloads with deterministic placeholders.
        #[arg(long)]
        gdpr_redact_emails: bool,
        /// Emit a JSON summary envelope instead of records.
        #[arg(long)]
        json: bool,
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
    /// audit_record.session_id -> session.vertical_id; rows from
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
        /// Lower bound on `seq` (inclusive).
        #[arg(long)]
        from_seq: Option<i64>,
        /// Upper bound (inclusive) on `created_at` as RFC-3339.
        #[arg(long)]
        until: Option<String>,
        /// Output format: `pretty` (default) or `raw` (NDJSON).
        #[arg(long, default_value = "pretty")]
        format: String,
    },
}
