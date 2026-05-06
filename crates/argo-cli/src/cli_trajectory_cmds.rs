//! `argo trajectory export` — emit successful action trajectories
//! in fine-tuning-friendly formats. The shape follows the existing
//! `argo audit export` pattern (since/until/format/output) so
//! operators don't have to learn a new flag vocabulary.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum TrajectoryCmd {
    /// Walk every action bundle under `.skycot/outputs/` and emit
    /// the captured trajectories in one of three formats:
    ///   * `anthropic` — `[{role, content, tool_use?}, ...]`
    ///   * `openai`    — `[{role, content, tool_calls?}, ...]`
    ///   * `jsonl`     — one raw trajectory object per line
    ///
    /// Use `--filter passed` to drop trajectories whose action
    /// state isn't `completed`. Pass `--include-screenshots` to
    /// bundle the per-step PNGs inline (base64) in the output;
    /// without it the trajectory carries `screenshot_relpath`
    /// pointers only.
    Export {
        /// RFC3339 timestamp or `YYYY-MM-DD` (UTC midnight) — drop
        /// trajectories whose `created_at` is older than this.
        #[arg(long)]
        since: Option<String>,
        /// RFC3339 timestamp or `YYYY-MM-DD` (UTC end-of-day) —
        /// drop trajectories whose `created_at` is newer than this.
        #[arg(long)]
        until: Option<String>,
        /// Output format. `anthropic` (default) / `openai` / `jsonl`.
        #[arg(long, default_value = "anthropic")]
        format: String,
        /// Trajectory filter. `passed` (default — only `completed`
        /// actions), `all` (every state including failed/blocked).
        #[arg(long, default_value = "passed")]
        filter: String,
        /// Inline per-step screenshots as base64-encoded PNG. Off
        /// by default — the path-manifest stays in
        /// `screenshot_relpath` so callers can fetch lazily.
        #[arg(long)]
        include_screenshots: bool,
        /// Apply the same email-redaction rules as
        /// `argo audit export --gdpr-redact-emails`.
        #[arg(long)]
        gdpr_redact_emails: bool,
        /// Write the export to this path. When omitted, prints to
        /// stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Emit a JSON envelope to stdout describing the export
        /// (rows, format, sha256). The export bytes still go to
        /// `--output` (or stdout if `--output` was omitted).
        #[arg(long)]
        json: bool,
    },
}
