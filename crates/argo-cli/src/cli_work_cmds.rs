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
        /// Emit JSON per job instead of human-readable.
        #[arg(long)]
        json: bool,
    },
}
