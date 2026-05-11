//! `argo skill versions / ab-test / ab-status / ab-promote`
//! subcommand args. Holds the four sibling Args structs so
//! `cli_subcommands.rs` only needs four single-line variants
//! (the cli_skill_search_cmds.rs precedent from
//! `codex-skill-marketplace-search`).

use clap::Args;

/// `argo skill versions <name>` — list every version of `<name>`
/// installed in this workspace, newest first.
#[derive(Args, Debug)]
pub struct SkillVersionsCmd {
    /// Skill name (the family identifier, NOT a version string).
    pub name: String,
    /// Emit JSON (one object per row) instead of a table.
    #[arg(long)]
    pub json: bool,
}

/// `argo skill ab-test <name> --variants vN,vM --runs N` —
/// create a new A/B test run pitting two existing versions of
/// `<name>` against each other.
#[derive(Args, Debug)]
pub struct SkillAbTestCmd {
    /// Skill name.
    pub name: String,
    /// Comma-separated variant pair: `<vA>,<vB>`. Both must
    /// already exist as `(name, version)` rows.
    #[arg(long)]
    pub variants: String,
    /// Target invocation count before the run is allowed to
    /// auto-complete on significance. Default 100.
    #[arg(long, default_value_t = 100)]
    pub runs: i64,
    /// Emit JSON (the new run row) instead of a one-line summary.
    #[arg(long)]
    pub json: bool,
}

/// `argo skill ab-status <run-id>` — print the run row + per-variant
/// counts + the chi-square verdict at this moment.
#[derive(Args, Debug)]
pub struct SkillAbStatusCmd {
    /// Run id (UUIDv7) returned by `argo skill ab-test`.
    pub run_id: String,
    /// Minimum samples per variant before significance is
    /// computed. Default 30 (per the brief).
    #[arg(long, default_value_t = 30)]
    pub min_samples: i64,
    /// Emit JSON instead of a human-readable summary.
    #[arg(long)]
    pub json: bool,
}

/// `argo skill ab-snapshot <skill_name>` — print the most recent
/// A/B run's per-variant counts + significance verdict for one
/// skill name. Operator inspection surface for "is this A/B done?"
/// added by mission-skill-select-variant-runtime-wire. Hyphenated to
/// match the existing `ab-test` / `ab-status` / `ab-promote`
/// pattern (the brief said `argo skill ab snapshot` but the rest
/// of the family is `ab-*`, so we follow the existing convention).
#[derive(Args, Debug)]
pub struct SkillAbSnapshotCmd {
    /// Skill name to inspect.
    pub name: String,
    /// Minimum samples per variant before significance is
    /// computed. Default 30 (matches `ab-status`).
    #[arg(long, default_value_t = 30)]
    pub min_samples: i64,
    /// Emit a JSON envelope instead of the human-readable summary.
    #[arg(long)]
    pub json: bool,
}

/// `argo skill ab-promote <run-id>` — apply a run's winner to the
/// `skill` table by stamping the loser's `superseded_by`. If the
/// run hasn't auto-completed, the operator must supply
/// `--winner <version>` explicitly.
#[derive(Args, Debug)]
pub struct SkillAbPromoteCmd {
    /// Run id.
    pub run_id: String,
    /// Override the auto-detected winner (required for runs that
    /// haven't reached significance yet).
    #[arg(long)]
    pub winner: Option<String>,
    /// Fraction (0.0..=1.0) of the auto-promote winner score
    /// contributed by the latest cached `QualityScore` per variant.
    /// `0.0` keeps today's success-rate-only behavior.
    #[arg(long, default_value_t = 0.3)]
    pub quality_weight: f32,
    /// Quality scores older than this are treated as missing (the
    /// variant falls through to success-rate-only logic).
    #[arg(long, default_value_t = 7)]
    pub max_score_age_days: u32,
    /// Minimum samples per variant before auto-promote will run
    /// the significance gate. Default 30.
    #[arg(long, default_value_t = 30)]
    pub min_samples: i64,
    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}
