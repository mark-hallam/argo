//! `argo work` — work queue v0 (#716). Two subcommands: queue a
//! unit of work for a vertical, and serve a persistent worker
//! loop that processes the queue.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkCmd {
    /// Queue a unit of work for a vertical-bound worker. Validates
    /// the vertical exists, then appends a `pending_jobs` memory
    /// entry under the vertical's namespace. Returns the job id
    /// (the memory entry's UUID).
    Add {
        /// Vertical id (e.g. `gtm-engineer`). Must be discoverable
        /// via `argo vertical list`.
        #[arg(long)]
        vertical: String,
        /// Free-form prompt describing the unit of work. Passed
        /// verbatim as the user message on the worker's next turn.
        #[arg(long)]
        prompt: String,
        /// Emit JSON instead of human-readable.
        #[arg(long)]
        json: bool,
    },
    /// Run a persistent worker loop. Reads the oldest unprocessed
    /// `pending_jobs` for the vertical, creates a fresh session,
    /// runs one turn, writes a `completed_jobs` memory entry, and
    /// repeats.
    ///
    /// Mock provider (default) produces deterministic synthetic
    /// output without dispatching the LLM — no API key required.
    /// Switch `--provider` / `--model` to drive a real provider
    /// (e.g. `--provider anthropic --model claude-sonnet-4-6`,
    /// requires `ANTHROPIC_API_KEY` in env).
    Serve {
        /// Vertical id this worker serves.
        #[arg(long)]
        vertical: String,
        /// Sleep between iterations when no work is pending.
        #[arg(long, default_value_t = 5)]
        interval_seconds: u64,
        /// Provider for new sessions. Default `mock` short-circuits
        /// to deterministic synthetic output without an API key.
        #[arg(long, default_value = "mock")]
        provider: String,
        /// Model for new sessions.
        #[arg(long, default_value = "mock")]
        model: String,
        /// Process at most one job batch then exit. Test affordance —
        /// production deployments should omit this.
        #[arg(long)]
        once: bool,
        /// Maximum number of multi-step worker turns per job (#722).
        /// One "step" is one full agent turn (i.e. one or more LLM
        /// rounds with tool dispatch, ending in an assistant text).
        /// The loop stops early when the model emits one of
        /// `[STOP: TASK_COMPLETE]`, `[STOP: APPROVAL_REQUIRED]`, or
        /// `[STOP: NEED_INFO ...]`. Setting `--max-steps 1` reverts
        /// to the single-turn Worker v1 behaviour and skips the
        /// research-tools pre-flight check.
        #[arg(long, default_value_t = 5)]
        max_steps: u32,
        /// Per-step ceiling on internal tool-dispatch rounds (#724).
        /// One "tool round" is one provider call plus its tool
        /// dispatch — research-heavy work (web_search ×N, then
        /// web_extract ×N) can hit the cap mid-turn. When that
        /// happens the worker no longer aborts with a transport
        /// error; it persists the partial transcript, ends the
        /// step cleanly, and continues to the next outer step.
        /// Default 8.
        #[arg(long, default_value_t = 8)]
        max_tool_rounds: usize,
        /// Emit JSON per job instead of human-readable.
        #[arg(long)]
        json: bool,
    },
    /// Surface one job's artifact bundle from disk without
    /// requiring the operator to remember the
    /// `.skycot/outputs/<job_id>/...` layout. Looks up the
    /// job's `output_dir` from its `completed_jobs` memory
    /// row, then prints (or writes to stdout) the requested
    /// file.
    Logs {
        /// The `job_id` (UUIDv7) of a previously-served job.
        /// Use `argo work jobs --json` to find a recent one.
        job_id: String,
        /// Which file to surface. Defaults to `transcript`.
        /// `all` prints a header per file followed by its
        /// contents, in canonical reading order.
        #[arg(
            long,
            default_value = "transcript",
            value_parser = ["transcript", "assistant", "approval_checklist", "execution_plan", "all"],
        )]
        kind: String,
    },
    /// List queued + completed worker jobs across the
    /// workspace. Walks the same `pending_jobs` /
    /// `completed_jobs` memory rows the queue runner uses, so
    /// what you see here is exactly what the worker would
    /// process / has processed.
    Jobs {
        /// Filter by job state. One of `pending`, `completed`,
        /// or `all`. Defaults to `all`.
        #[arg(long, default_value = "all")]
        state: String,
        /// Filter to a specific vertical's namespace. When
        /// omitted, walks every loaded vertical.
        #[arg(long)]
        vertical: Option<String>,
        /// Cap the number of rows printed. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit JSON-lines (one row per job) instead of a
        /// human-readable table.
        #[arg(long)]
        json: bool,
    },
}
