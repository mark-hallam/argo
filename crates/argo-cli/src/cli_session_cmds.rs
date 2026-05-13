//! Session / forge / judge / route / import command enums.
//! Extracted from `cli_subcommands.rs` to keep that file under
//! the 500-LOC budget. Re-exported by `cli_subcommands.rs` so
//! existing `use crate::cli::SessionCmd` callers stay unchanged.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SessionCmd {
    /// Create a session row directly (orchestrator path).
    #[command(after_help = "Examples:\n  \
        argo session start --title \"morning standup notes\"\n  \
        argo session start --title \"design review\" --worker scout --json")]
    Start {
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long)]
        worker: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List sessions with rich filters (status, provider, model,
    /// title-substring, spend window, etc.).
    #[command(after_help = "Examples:\n  \
        argo session list\n  \
        argo session list --status active --limit 50\n  \
        argo session list --since-minutes 60 --provider anthropic --json")]
    List {
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: u32,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "text")]
        format: String,
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        older_than_minutes: Option<i64>,
        #[arg(long)]
        spent_min: Option<u64>,
        #[arg(long)]
        spent_max: Option<u64>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long, default_value = "all")]
        status: String,
        #[arg(long)]
        title_contains: Option<String>,
        #[arg(long, default_value = "started-at")]
        sort_by: String,
        #[arg(long)]
        vertical: Option<String>,
    },
    Count {
        #[arg(long, default_value = "all")]
        status: String,
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        older_than_minutes: Option<i64>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        title_contains: Option<String>,
        #[arg(long)]
        spent_min: Option<u64>,
        #[arg(long)]
        spent_max: Option<u64>,
        #[arg(long)]
        json: bool,
    },
    /// Stream a session's new messages live (poll loop).
    #[command(after_help = "Examples:\n  \
        argo session watch 01J9XYZ...")]
    Watch {
        id: String,
    },
    /// Render the message log for one session.
    #[command(after_help = "Examples:\n  \
        argo session messages 01J9XYZ...\n  \
        argo session messages 01J9XYZ... --role assistant --tail 20\n  \
        argo session messages 01J9XYZ... --json")]
    Messages {
        id: String,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "text")]
        format: String,
        #[arg(long, conflicts_with = "tail")]
        limit: Option<u32>,
        #[arg(long)]
        tail: Option<u32>,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        seq_min: Option<i64>,
        #[arg(long)]
        seq_max: Option<i64>,
        #[arg(long)]
        content_contains: Option<String>,
    },
    AttachSkill {
        id: String,
        skill: String,
    },
    DetachSkill {
        id: String,
        skill: String,
    },
    Skills {
        id: String,
    },
    ClearSkills {
        id: String,
    },
    /// Print the session header (provider, model, status, spend).
    #[command(after_help = "Examples:\n  \
        argo session show 01J9XYZ...\n  \
        argo session show 01J9XYZ... --json")]
    Show {
        id: String,
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    Transcript { id: String, #[arg(long)] json: bool },
    Export {
        id: String,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        include_audit: bool,
        #[arg(long, default_value = "jsonl")]
        format: String,
    },
    /// Full-text search across session messages.
    #[command(after_help = "Examples:\n  \
        argo session search \"timeout\"\n  \
        argo session search \"approval denied\" --role assistant --limit 5 --json\n  \
        argo session search \"oom\" --count")]
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "text")]
        format: String,
        #[arg(long, conflicts_with_all = ["limit", "format"])]
        count: bool,
    },
    /// Delete a session row + its messages (use `--force` to skip the prompt).
    #[command(after_help = "Examples:\n  \
        argo session delete 01J9XYZ...\n  \
        argo session delete 01J9XYZ... --force")]
    Delete {
        id: String,
        #[arg(long)]
        force: bool,
    },
    Replay {
        id: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        tools: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        stream: bool,
    },
    Diff {
        id1: String,
        id2: String,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    Title {
        id: String,
        #[arg(required_unless_present = "clear")]
        new: Option<String>,
        #[arg(long, conflicts_with = "new")]
        clear: bool,
    },
    End { id: String },
    Reopen { id: String },
    EndAll {
        #[arg(long)]
        older_than_minutes: Option<i64>,
    },
    Bisect {
        id: String,
        hypothesis: String,
    },
    Clone {
        id: String,
    },
    Branch {
        id: String,
        #[arg(long)]
        through_seq: Option<i64>,
    },
    Stats {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Turns {
        id: String,
        #[arg(long)]
        json: bool,
    },
    ByDay {
        #[arg(long, default_value_t = 30)]
        days: u32,
        #[arg(long)]
        json: bool,
    },
    ByProvider {
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    ByModel {
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    AuditByKind {
        id: String,
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    AuditByTool {
        id: String,
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    MessagesByRole {
        id: String,
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    ToolErrors {
        id: String,
        #[arg(long)]
        since_minutes: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    Approve {
        session_id: String,
        #[arg(long)]
        tool: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        once: bool,
        #[arg(long)]
        json: bool,
    },
    RevokeApproval {
        session_id: String,
        #[arg(long)]
        tool: String,
        #[arg(long)]
        json: bool,
    },
    ListGrants {
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    CapabilityGaps {
        session_id: String,
        #[arg(long, default_value_t = 0)]
        since_minutes: i64,
        #[arg(long, default_value_t = 3)]
        threshold: u64,
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
    /// Run the judge-vs-human disagreement calibration suite over a
    /// directory of pre-labelled fixtures. Reports agreement rate
    /// overall and per tag bucket. With `--bail-below <rate>`, exits
    /// non-zero when the overall rate falls below the threshold —
    /// suitable for CI gating on judge-model upgrades. The default
    /// threshold of 0.85 is a starting point; tune per-tenant.
    /// The judge transport is built with `temperature = 0` for
    /// reproducibility — see `argo-judge::eval_suite` for details.
    EvalDisagreement {
        /// Directory of `*.json` fixtures. Defaults to the bundled
        /// `crates/argo-judge/fixtures/disagreement/` directory.
        #[arg(long)]
        fixtures: Option<std::path::PathBuf>,
        /// Provider manifest id used to construct the judge transport.
        #[arg(long, default_value = "anthropic")]
        provider: String,
        /// Judge model id (e.g. `claude-sonnet-4-6`).
        #[arg(long, default_value = "claude-sonnet-4-6")]
        model: String,
        /// Emit a structured JSON envelope instead of a text summary.
        #[arg(long)]
        json: bool,
        /// Exit non-zero if the overall agreement rate is strictly
        /// below this value. Range `[0.0, 1.0]`. Default `0.85`.
        #[arg(long, default_value_t = 0.85)]
        bail_below: f32,
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
    Validate { path: String },
}

#[derive(Subcommand, Debug)]
pub enum HandoffCmd {
    List {
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        worker: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
    Show {
        id: String,
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Cancel { id: String, #[arg(long)] tenant: Option<String>, #[arg(long)] note: Option<String>, #[arg(long)] json: bool },
}

// `MessagesCmd` was extracted to cli_messages_cmds.rs (#624)
// when this file went over the 500-LOC budget. Re-exported by
// the wildcard glob in cli.rs so callers don't need to know
// the new path.
pub use super::cli_messages_cmds::MessagesCmd;
