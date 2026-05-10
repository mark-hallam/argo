//! `argo eval ...` subcommands.
//!
//! Top-level eval commands cover two shapes: the legacy generic
//! task JSONL validator and the vertical-fixture runner/listing
//! surface used by Worker OS eval seeds.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum EvalCmd {
    /// Validate eval seeds. Accepts either a JSONL path (legacy
    /// generic task format) or a vertical id; the latter loads the
    /// vertical's eval suites and reports a pass-rate envelope
    /// (`mvp-vertical-eval-harness`).
    Validate {
        /// Either a path to a JSONL fixtures file, or the id of a
        /// vertical loadable from `$ARGO_VERTICALS_DIR`/`./verticals`.
        target: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Run a vertical's eval fixtures through the dry-run worker
    /// pipeline and report pass/fail totals.
    Run {
        /// Vertical id to load from $ARGO_VERTICALS_DIR or ./verticals.
        #[arg(long)]
        vertical: String,
        /// Also score every fixture with the deterministic judge
        /// rules exposed by argo-vertical.
        #[arg(long)]
        score: bool,
        /// Cap the number of fixtures executed, after manifest
        /// suite order is applied.
        #[arg(long)]
        limit: Option<usize>,
        /// Streaming output mode (Stream `claude-eval-streaming`):
        /// `off` (batch — historical default), `simple` (default
        /// — one line per fixture as it completes), `verbose`
        /// (NDJSON per fixture for piping). Combine with `--json`
        /// to skip per-fixture lines and emit only the JSON
        /// envelope at the end.
        #[arg(long, default_value = "simple")]
        stream: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// List recent eval runs. With `--vertical`, filters to one vertical.
    List {
        /// Vertical id to filter by.
        #[arg(long)]
        vertical: Option<String>,
        /// Maximum number of runs to return.
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Show a single eval run.
    Show {
        /// Eval run id or path to an eval run JSON file.
        run: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Compare two eval runs side by side.
    Compare {
        /// Baseline eval run id or JSON path.
        run_a: String,
        /// Candidate eval run id or JSON path.
        run_b: String,
        /// Group deltas by fixture, skill, or metric.
        #[arg(long, default_value = "fixture")]
        by: String,
        /// Show only fixtures worse than run A in markdown output.
        #[arg(long)]
        show_regressions: bool,
        /// Show only fixtures better than run A in markdown output.
        #[arg(long)]
        show_improvements: bool,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
}
