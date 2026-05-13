//! Quality-side skill subcommand argument structs.
//!
//! Extracted as the second-pass cohesive cluster under the
//! `claude-cli-subcommands-skill-split` brief (parent file was at
//! 596 LOC). The four `Quality*` variants converted from inline
//! `Variant { fields }` form to `Variant(Cmd)` tuple form, matching
//! the established pattern used by `Search`, `Sign`, and the
//! `Versions`/`AbTest`/`AbStatus`/`AbPromote`/`AbSnapshot`
//! sibling-file `*Cmd` types.
use clap::Args;

#[derive(Args, Debug)]
pub struct SkillQualityCmd {
    /// Skill name (active version is used).
    pub name: String,
    /// Emit JSON. Equivalent to `--format json`.
    #[arg(long)]
    pub json: bool,
    /// Output format: `text` (default), `markdown`, or `json`.
    /// `--json` wins if both are passed.
    #[arg(long, default_value = "text")]
    pub format: String,
    /// Render history of the last N scores rather than only the
    /// latest. Default 1 (latest only).
    #[arg(long, default_value_t = 1)]
    pub limit: u32,
}

#[derive(Args, Debug)]
pub struct SkillQualityListCmd {
    /// Optional ceiling — only rows with `score <=
    /// threshold` are emitted.
    #[arg(long)]
    pub threshold: Option<u8>,
    /// Truncate to the lowest-scoring N rows.
    #[arg(long)]
    pub worst: Option<u32>,
    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillQualityTrendCmd {
    /// Skill name (active version is used).
    pub name: String,
    /// Days to render (default 30, max 365).
    #[arg(long)]
    pub days: Option<u32>,
    /// Emit JSON envelope (per-day buckets + sparkline).
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillQualityRecomputeCmd {
    /// Recompute one named skill. Mutually exclusive with `--all`.
    ///
    /// (Sensible deviation from the brief, which specified
    /// `<skill_id>`: CLI commands consistently take a skill
    /// name, so this flag accepts the active skill's name.
    /// The output still prints the underlying `SkillId`.)
    #[arg(long, conflicts_with = "all", required_unless_present = "all")]
    pub skill: Option<String>,
    /// Recompute every active skill in the local store.
    #[arg(long, conflicts_with = "skill")]
    pub all: bool,
    /// Only consider audit rows newer than this cutoff.
    /// Accepts an RFC-3339 timestamp (`2026-01-01T00:00:00Z`)
    /// or a relative duration (`30m`, `24h`, `7d`).
    #[arg(long)]
    pub since: Option<String>,
    /// Compute + print but do NOT persist the new score.
    #[arg(long)]
    pub dry_run: bool,
}
