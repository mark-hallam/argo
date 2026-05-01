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
        #[arg(long)]
        json: bool,
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
    /// Show one session header (model, provider, start time, message count).
    Show {
        id: String,
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Dump a session's messages to JSONL (defaults to stdout;
    /// pass `--out <path>` to write to a file).
    Export {
        id: String,
        #[arg(long)]
        out: Option<String>,
        /// Append audit rows for the session after the messages.
        /// Each audit line has the standard `ExportedRow` shape;
        /// distinguishable from message lines by the `kind` field
        /// (audit rows) vs `role` (message rows).
        #[arg(long)]
        include_audit: bool,
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
    },
    /// Structural diff of two recorded sessions.
    Diff {
        id1: String,
        id2: String,
        /// Output as a single JSON object for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Mark a session ended (stamps `ended_at`). Idempotent.
    End { id: String },
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
}

#[derive(Subcommand, Debug)]
pub enum ForgeCmd {
    Optimize {
        skill: String,
        #[arg(long)]
        examples: String,
        #[arg(long, default_value_t = 12)]
        iterations: u32,
        #[arg(long, default_value_t = 5.0)]
        budget: f64,
        /// Emit progress as newline-delimited JSON on stdout
        /// instead of the human-readable table. Each line is
        /// either `{"event":"start", run_id, ...}`,
        /// `{"event":"iter", iter, version, mean_score, …}`, or
        /// `{"event":"done", best_score, best_version, status}`.
        /// Lets the TUI / watch scripts / gateway consume forge
        /// progress without parsing the table.
        #[arg(long)]
        stream: bool,
    },
    /// Run an eval-task JSONL file through the agent loop and
    /// score each task. Records a `forge_run` row with the mean
    /// score + pass rate when the run finishes. Skill name is
    /// captured for the row's metadata; the active skill body
    /// isn't (yet) injected into the prompt — that lands with
    /// the daemon-side prompt-loader PR.
    Bench {
        skill: String,
        /// Path to a JSONL eval-task file (parsed by `argo eval
        /// validate`).
        #[arg(long)]
        examples: String,
        /// Provider id (matches `manifests/providers/<id>.toml`).
        /// Defaults to `anthropic`.
        #[arg(long, default_value = "anthropic")]
        provider: String,
        /// Model id (matches an entry in the manifest's
        /// `[[models]]` list). Defaults to
        /// `claude-3-5-sonnet-20241022`.
        #[arg(long, default_value = "claude-3-5-sonnet-20241022")]
        model: String,
    },
    Promote {
        skill: String,
        #[arg(long)]
        version: String,
    },
    Runs {
        #[command(subcommand)]
        cmd: ForgeRunsCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum ForgeRunsCmd {
    /// List recorded forge runs. Mirrors the gateway's
    /// `/forge/runs` endpoint with the same filter set.
    List {
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Output as JSON (one object per row).
        #[arg(long)]
        json: bool,
        /// Filter to runs started within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Status filter: running|succeeded|failed|aborted.
        #[arg(long)]
        status: Option<String>,
        /// Inclusive lower bound on `final_score`.
        #[arg(long)]
        score_min: Option<f64>,
        /// Inclusive upper bound on `final_score`.
        #[arg(long)]
        score_max: Option<f64>,
        /// Exact-match filter on skill_name.
        #[arg(long)]
        skill: Option<String>,
        /// `true` to keep only rows with a non-NULL
        /// candidate_version.
        #[arg(long)]
        has_candidate: Option<bool>,
    },
    Show { id: String },
    Abort { id: String },
}

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
