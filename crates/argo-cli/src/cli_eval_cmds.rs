//! `argo eval ...` subcommands.
//!
//! Top-level eval commands cover two shapes: the legacy generic
//! task JSONL validator and the vertical-fixture runner/listing
//! surface used by Worker OS eval seeds.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum EvalCmd {
    /// Validate eval seeds. Accepts either a JSONL (JSON Lines —
    /// one JSON object per line) path (legacy generic task format)
    /// or a vertical id; the latter loads the vertical's eval suites
    /// and reports a pass-rate envelope (`mvp-vertical-eval-harness`).
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
        /// (NDJSON — newline-delimited JSON, one object per line —
        /// per fixture for piping). Combine with `--json` to skip
        /// per-fixture lines and emit only the JSON envelope at the
        /// end.
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
    /// Self-evaluation honesty test: sample recent skill runs, re-judge
    /// them via `argo-judge::adjudicate`, and flag the skill as
    /// SUSPICIOUS when its self-reported success rate diverges from the
    /// independent judge by more than the configured threshold
    /// (mission-eval-self-honesty-test). Diagnostic only — does not
    /// demote trust grades.
    SkillHonesty {
        /// Skill id to audit.
        skill_id: String,
        /// Override the default 20-sample window.
        #[arg(long)]
        sample: Option<usize>,
        /// Provider manifest id used to construct the judge transport.
        #[arg(long, default_value = "anthropic")]
        provider: String,
        /// Judge model id (e.g. `claude-sonnet-4-6`).
        #[arg(long, default_value = "claude-sonnet-4-6")]
        model: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Export a tenant's real *judged* history (human approval
    /// decisions + committed retrains) as SFT-style JSONL training
    /// data. Read-only; closes competitive gap #3 (learn from the
    /// audit record). See
    /// `docs/skycot/design/2026-05-29-gap-closure-plans/03-self-improvement-training-export.md`.
    ExportTrainingData {
        /// Tenant id (UUID) to export for.
        #[arg(long)]
        tenant: String,
        /// Output file path (JSONL — one example per line).
        #[arg(long, short)]
        out: std::path::PathBuf,
        /// Emit a single JSON summary object instead of human lines.
        #[arg(long)]
        json: bool,
    },
    /// Generate `argo-eval` regression cases from a tenant's real
    /// *judged* history (self-improvement Step 1b): each human-rejected
    /// approval becomes a "must refuse or defer" case, each committed
    /// retrain a "corrected behaviour" case. Read-only; the output
    /// loads + runs via `argo eval validate <file>`. See
    /// `docs/skycot/design/2026-05-29-gap-closure-plans/03-self-improvement-training-export.md`.
    GenerateCases {
        /// Tenant id (UUID) to generate cases for.
        #[arg(long)]
        tenant: String,
        /// Output file path (JSONL — one `EvalTask` per line).
        #[arg(long, short)]
        out: std::path::PathBuf,
        /// Emit a single JSON summary object instead of human lines.
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
    /// Close the self-improvement loop: run TWO versions of one skill
    /// through the real eval scorer and report the measured pass-rate
    /// delta (the "skill improved from X% → Y%" proof, DEC-023). Both
    /// bodies are scored against the candidate's declared `evals` (or the
    /// baseline's if the candidate declares none); the deterministic
    /// body-aware executor returns each version's body, so a `contains`
    /// predicate genuinely passes only when THAT body carries the
    /// required guidance — the delta is a real measurement, not a rig.
    /// Persists the result to `skill_reeval_history` so the worker Evals
    /// tab (`GET /api/v1/workers/<id>/evals`) shows the candidate's run.
    SkillAb {
        /// Skill family name (both versions must exist under it).
        skill: String,
        /// Baseline ("before") version string, e.g. `v1`.
        #[arg(long)]
        baseline: String,
        /// Candidate ("after") version string, e.g. `v2`.
        #[arg(long)]
        candidate: String,
        /// Record the candidate's measured pass rate into
        /// `skill_reeval_history` (so the worker Evals tab reflects it).
        #[arg(long)]
        record: bool,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
}
