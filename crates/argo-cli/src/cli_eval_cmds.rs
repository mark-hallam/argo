//! `argo eval ...` subcommands.
//!
//! Top-level eval commands cover two shapes: the legacy generic
//! task JSONL validator and the vertical-fixture runner/listing
//! surface used by Worker OS eval seeds.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum EvalCmd {
    /// Validate a generic argo-eval task JSONL file.
    Validate {
        /// Path to a JSONL file containing argo-eval tasks.
        path: String,
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
    /// List the fixtures declared by a vertical's eval suites.
    List {
        /// Vertical id to load from $ARGO_VERTICALS_DIR or ./verticals.
        #[arg(long)]
        vertical: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
}
