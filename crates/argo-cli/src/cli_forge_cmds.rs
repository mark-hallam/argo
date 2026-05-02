//! `ForgeCmd` + `ForgeRunsCmd` clap subcommand enums. Lifted
//! out of `cli_session_cmds.rs` so that file stays under the
//! 500-LOC ceiling as forge gains the daemon variant.
use clap::Subcommand;

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
    /// Enqueue a forge optimize run for the v1.1 daemon to
    /// pick up later. Creates a `forge_run` row with
    /// status='queued' and a parallel `forge_queue` payload.
    /// Returns the new run id; the daemon claims + runs it on
    /// its next tick. Use `argo forge runs list --status queued`
    /// to see what's waiting.
    Enqueue {
        skill: String,
        #[arg(long)]
        examples: String,
        #[arg(long, default_value_t = 12)]
        iterations: u32,
        #[arg(long, default_value_t = 5.0)]
        budget: f64,
        /// Output the new run id as JSON instead of plain text.
        #[arg(long)]
        json: bool,
    },
    /// Long-running standalone daemon — periodically sweeps
    /// stalled `running` forge_run rows to `failed`. First step
    /// in the v1.1 forge-daemon split: replaces the manual
    /// `argo forge runs sweep` cadence with a worker that
    /// survives gateway restarts. Future iterations will pick up
    /// queued rows and run them.
    Daemon {
        /// Seconds between sweep ticks (default 60).
        #[arg(long, default_value_t = 60)]
        interval_secs: u64,
        /// Stalled-row cutoff in minutes (default 60). Rows
        /// stuck in `running` older than this get flipped to
        /// `failed` on each tick.
        #[arg(long, default_value_t = 60)]
        older_than_minutes: i64,
        /// Stop after this many ticks (test/CI hook). Default
        /// runs forever.
        #[arg(long)]
        max_ticks: Option<u64>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ForgeRunsCmd {
    /// Print just the count of recorded forge runs.
    Count {
        /// Filter by status: running|succeeded|failed|aborted.
        #[arg(long)]
        status: Option<String>,
        /// Filter to runs started within this many recent minutes.
        /// Mirrors the gateway's `?since-minutes=N`.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object {count, status?}.
        #[arg(long)]
        json: bool,
    },
    /// List recorded forge runs. Mirrors the gateway's
    /// `/forge/runs` endpoint with the same filter set.
    List {
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Output as JSON (one object per row).
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`
        /// (renders a markdown table). `--json` wins if both are
        /// passed.
        #[arg(long, default_value = "text")]
        format: String,
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
        /// Inclusive lower bound on `iterations`. Mirrors
        /// the gateway's `?iterations-min=N`.
        #[arg(long)]
        iterations_min: Option<i64>,
        /// Inclusive upper bound on `iterations`. Mirrors
        /// the gateway's `?iterations-max=N`.
        #[arg(long)]
        iterations_max: Option<i64>,
        /// Sort order: `started-at` (default, descending),
        /// `score` (final_score descending; NULLs sink to the
        /// bottom), or `spend` (spend_usd_micros descending).
        #[arg(long, default_value = "started-at")]
        sort_by: String,
    },
    Show { id: String },
    Abort { id: String },
    /// Requeue a terminal forge run (failed / aborted /
    /// succeeded) by copying its queued payload into a fresh
    /// row with status='queued'. The daemon picks it up on
    /// its next tick. The source row stays as a historical
    /// record. 404 when the source has no payload (was run
    /// via `?in_process=true`). Mirrors the gateway's
    /// `POST /forge/runs/<id>/requeue`.
    Requeue {
        id: String,
        /// Output as a single JSON object {requeued, source_id, run_id}.
        #[arg(long)]
        json: bool,
    },
    /// Flip every running forge run to `aborted`. Operator
    /// emergency-stop / fresh-slate before a workspace migration.
    /// Mirrors `POST /forge/runs/abort-all`.
    AbortAll,
    /// Sweep stalled `running` rows whose `started_at` is older
    /// than `--older-than-minutes` ago (default 60). Used to
    /// recover from a gateway crash mid-optimize: in-memory
    /// state is gone, but the row stays in `running` until
    /// manually aborted. Flips status to `failed`. Mirrors
    /// `POST /forge/runs/sweep`.
    Sweep {
        /// Cutoff in minutes. Default 60 — runs older than this
        /// that are still in `running` get flipped to `failed`.
        #[arg(long, default_value_t = 60)]
        older_than_minutes: i64,
    },
    /// Daily forge-run start rollup. Mirrors the gateway's
    /// `GET /forge/runs/by-day` (#572).
    ByDay {
        /// Lookback window in days (default 30, cap 365).
        #[arg(long, default_value_t = 30)]
        days: u32,
        /// Output as a single JSON object (`{by_day}`).
        #[arg(long)]
        json: bool,
    },
    /// Histogram of forge_run rows grouped by `status`.
    /// Mirrors the gateway's `GET /forge/runs/by-status` (#584).
    ByStatus {
        /// Filter to runs started within this many recent
        /// minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object (`{by_status}`).
        #[arg(long)]
        json: bool,
    },
}
