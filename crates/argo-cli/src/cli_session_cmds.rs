//! Session / forge / judge / route / import command enums.
//! Extracted from `cli_subcommands.rs` to keep that file under
//! the 500-LOC budget. Re-exported by `cli_subcommands.rs` so
//! existing `use crate::cli::SessionCmd` callers stay unchanged.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SessionCmd {
    /// List recent sessions for the current tenant.
    List {
        #[arg(long, default_value_t = 25)]
        limit: u32,
        /// Output as JSON (one object per session) for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`
        /// (renders a markdown table). `--json` wins if both are
        /// passed.
        #[arg(long, default_value = "text")]
        format: String,
        /// Filter to sessions started within this many recent
        /// minutes (e.g. `--since 60` for the last hour). Compared
        /// against `started_at`.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Inverse of --since-minutes: keep only sessions
        /// whose `started_at` is older than N minutes ago.
        /// Useful for prune candidates.
        #[arg(long)]
        older_than_minutes: Option<i64>,
        /// Lower bound on per-session spend in usd_micros.
        /// `--spent-min 1000000` keeps only sessions costing $1+.
        #[arg(long)]
        spent_min: Option<u64>,
        /// Upper bound on per-session spend in usd_micros.
        #[arg(long)]
        spent_max: Option<u64>,
        /// Exact-match filter on session.provider, e.g.
        /// `--provider anthropic`.
        #[arg(long)]
        provider: Option<String>,
        /// Exact-match filter on session.model, e.g.
        /// `--model claude-sonnet-4.5`.
        #[arg(long)]
        model: Option<String>,
        /// Status filter: `open` keeps sessions with a NULL
        /// `ended_at`, `ended` keeps the rest, `all` (default)
        /// returns both. Mirrors the gateway's `?status=` query.
        #[arg(long, default_value = "all")]
        status: String,
        /// Case-insensitive substring filter on session.title.
        #[arg(long)]
        title_contains: Option<String>,
        /// Sort order: `started-at` (default, descending) or
        /// `spend` (descending — biggest spenders first).
        #[arg(long, default_value = "started-at")]
        sort_by: String,
    },
    /// Print just the count of sessions matching the filters
    /// (useful for monitoring scripts). One number per line, or
    /// a single JSON object with --json.
    Count {
        /// Status filter: `open` / `ended` / `all` (default).
        #[arg(long, default_value = "all")]
        status: String,
        /// Filter to sessions newer than N minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Inverse of --since-minutes.
        #[arg(long)]
        older_than_minutes: Option<i64>,
        /// Exact-match filter on session.provider.
        #[arg(long)]
        provider: Option<String>,
        /// Exact-match filter on session.model.
        #[arg(long)]
        model: Option<String>,
        /// Case-insensitive substring filter on session.title.
        /// Mirrors `argo session list --title-contains`.
        #[arg(long)]
        title_contains: Option<String>,
        /// Lower bound on per-session spend in usd_micros.
        #[arg(long)]
        spent_min: Option<u64>,
        /// Upper bound on per-session spend in usd_micros.
        #[arg(long)]
        spent_max: Option<u64>,
        /// Output as a single JSON object {count, status, ...}
        /// for scripting.
        #[arg(long)]
        json: bool,
    },
    /// `tail -f`-style live watch of new messages on a session.
    /// Polls every 500ms; prints each newly-appended row as a
    /// one-liner (`[seq] role: text-summary`). Exit with Ctrl-C.
    Watch {
        id: String,
    },
    /// List the messages on a session. Mirrors the gateway's
    /// `GET /sessions/<id>/messages` query surface (limit, role,
    /// since-minutes, seq-min/max, content-contains).
    Messages {
        id: String,
        /// Output as JSON (one object per message) for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`
        /// (one `## [seq] role` block per message). `--json`
        /// wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
        /// Truncate to the first N rows after filtering.
        #[arg(long, conflicts_with = "tail")]
        limit: Option<u32>,
        /// Take the LAST N rows after filtering (inverse of
        /// `--limit`). Useful for `tail -f`-style inspection of
        /// the most recent activity.
        #[arg(long)]
        tail: Option<u32>,
        /// Filter to one role: `user|assistant|tool|system`.
        #[arg(long)]
        role: Option<String>,
        /// Keep messages created within the last N minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Lower bound on `seq` (inclusive).
        #[arg(long)]
        seq_min: Option<i64>,
        /// Upper bound on `seq` (inclusive).
        #[arg(long)]
        seq_max: Option<i64>,
        /// Case-insensitive substring filter on the raw
        /// `content_json` blob (catches user prompts, assistant
        /// text, tool args/results in one pass).
        #[arg(long)]
        content_contains: Option<String>,
    },
    /// Attach an active skill to a session. While at least one
    /// skill is attached, the agent runner's system-prompt loader
    /// uses *only* the attached set instead of every active skill.
    AttachSkill {
        id: String,
        skill: String,
    },
    /// Detach a skill from a session. With no remaining
    /// attachments, the loader falls back to all active skills.
    DetachSkill {
        id: String,
        skill: String,
    },
    /// List the skills attached to a session (one per line).
    Skills {
        id: String,
    },
    /// Detach every attached skill from a session — reset to
    /// fall-through-all-active-skills without having to list the
    /// attached set first. Mirrors
    /// `POST /sessions/<id>/skills/clear`.
    ClearSkills {
        id: String,
    },
    /// Show one session header (model, provider, start time, message count).
    Show {
        id: String,
        /// Output as a single JSON object for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Dump a session's messages (defaults to stdout; pass
    /// `--out <path>` to write to a file). Default format is
    /// JSONL (one self-describing object per line); pass
    /// `--format markdown` for a human-readable transcript.
    Export {
        id: String,
        #[arg(long)]
        out: Option<String>,
        /// Append audit rows for the session after the messages.
        /// Each audit line has the standard `ExportedRow` shape;
        /// distinguishable from message lines by the `kind` field
        /// (audit rows) vs `role` (message rows). Only applies to
        /// `--format jsonl`.
        #[arg(long)]
        include_audit: bool,
        /// Output format: `jsonl` (default) or `markdown`.
        #[arg(long, default_value = "jsonl")]
        format: String,
    },
    /// Full-text search the message corpus via FTS5. Pass
    /// `--session <id>` to scope to one session.
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        session: Option<String>,
        /// Filter hits to one role: user|assistant|tool|system.
        /// Mirrors the gateway's `?role=` filter (#414).
        #[arg(long)]
        role: Option<String>,
        /// Output as JSON (one object per hit).
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`
        /// (renders a markdown table). `--json` wins if both are
        /// passed.
        #[arg(long, default_value = "text")]
        format: String,
        /// Print just the match count instead of the row bodies.
        /// Mirrors the gateway's `GET /search/count` (#559).
        /// Caps the underlying fetch at 10000; when the cap is
        /// hit the count is reported as `N+` (text) or
        /// `truncated: true` (json).
        #[arg(long, conflicts_with_all = ["limit", "format"])]
        count: bool,
    },
    /// Delete a session and all its dependent rows. Pass `--force`
    /// to skip the confirmation prompt.
    Delete {
        id: String,
        #[arg(long)]
        force: bool,
    },
    /// Replay a recorded session's user messages into a fresh
    /// session against a (possibly different) model. Each user
    /// message is appended in order and the agent loop runs one
    /// turn per message; the new session id is printed at the end.
    Replay {
        id: String,
        /// Override the model. Defaults to the source session's
        /// model.
        #[arg(long)]
        model: Option<String>,
        /// Override the provider id (matches `manifests/providers/
        /// <id>.toml`). Defaults to the source session's provider.
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        tools: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        /// Stream per-turn `RunEvent`s as newline-delimited JSON
        /// on stdout (one event per line) and finish with a
        /// `{"event":"done", ...}` line. Mirrors the gateway's
        /// `POST /sessions/<id>/replay/stream` chunked response.
        #[arg(long)]
        stream: bool,
    },
    /// Structural diff of two recorded sessions.
    Diff {
        id1: String,
        id2: String,
        /// Output as a single JSON object for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Set or clear a session's title. Pass `--clear` to remove
    /// the title, or `<new>` to set a fresh one. Mirrors the
    /// gateway's `POST /sessions/<id>/title`.
    Title {
        id: String,
        /// New title. Required unless `--clear` is set.
        #[arg(required_unless_present = "clear")]
        new: Option<String>,
        /// Clear the title (set to NULL).
        #[arg(long, conflicts_with = "new")]
        clear: bool,
    },
    /// Mark a session ended (stamps `ended_at`). Idempotent.
    End { id: String },
    /// Reopen an ended session (clears `ended_at`). Idempotent.
    Reopen { id: String },
    /// Bulk-mark every open session as ended. Without
    /// `--older-than-minutes`, the operator-emergency-stop
    /// behaviour applies (every open session gets ended).
    EndAll {
        /// Only end sessions whose `started_at` is older than
        /// N minutes ago — periodic cleanup of stale sessions.
        #[arg(long)]
        older_than_minutes: Option<i64>,
    },
    Bisect {
        id: String,
        hypothesis: String,
    },
    /// Duplicate a session into a fresh row, copying every
    /// message verbatim. Mirrors the gateway's
    /// `POST /sessions/<id>/clone`.
    Clone {
        /// Source session id to clone from.
        id: String,
    },
    /// Fork a session at a specific message — copies messages
    /// 1..=through-seq into a fresh row. Without
    /// `--through-seq`, copies every message (equivalent to
    /// `clone` but with a "(branch)" title suffix).
    Branch {
        /// Source session id to branch from.
        id: String,
        /// Inclusive seq to copy through.
        #[arg(long)]
        through_seq: Option<i64>,
    },
    /// Per-session summary view: role histogram + timing
    /// envelope + inter-message gap stats. Mirrors the
    /// gateway's `GET /sessions/<id>/stats`.
    Stats {
        /// Session id to summarise.
        id: String,
        /// Output as JSON (mirrors the gateway endpoint shape).
        #[arg(long)]
        json: bool,
    },
    /// Daily session-creation rollup — chronological list of
    /// `{day, count}` for the last `--days` days. Mirrors the
    /// gateway's `GET /sessions/by-day` endpoint.
    ByDay {
        /// Lookback window in days (default 30, cap 365).
        #[arg(long, default_value_t = 30)]
        days: u32,
        /// Output as a single JSON object (`{by_day}`).
        #[arg(long)]
        json: bool,
    },
    /// Histogram of sessions grouped by provider id. Mirrors
    /// the gateway's `GET /sessions/by-provider` (#594).
    ByProvider {
        /// Filter to sessions started within this many recent
        /// minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object (`{by_provider}`).
        #[arg(long)]
        json: bool,
    },
}

// `ForgeCmd` + `ForgeRunsCmd` live in `cli_forge_cmds.rs` —
// extracted to keep this file under the 500-LOC ceiling once
// `ForgeCmd::Daemon` was added for the v1.1 daemon split.
pub use super::cli_forge_cmds::{ForgeCmd, ForgeRunsCmd};

#[derive(Subcommand, Debug)]
pub enum JudgeCmd {
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
pub enum RouteCmd {
    Inspect { session_id: String },
    Bench,
}

#[derive(Subcommand, Debug)]
pub enum ImportCmd {
    Hermes { path: String },
}

#[derive(Subcommand, Debug)]
pub enum EvalCmd {
    /// Validate an eval task JSONL file: parse every row, report
    /// per-line errors and the metric histogram.
    Validate { path: String },
}
