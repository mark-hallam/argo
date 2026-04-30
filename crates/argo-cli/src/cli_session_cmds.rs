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
    },
    Bench {
        skill: String,
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
    List,
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
