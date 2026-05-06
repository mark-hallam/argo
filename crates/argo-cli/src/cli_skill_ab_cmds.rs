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
    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}
