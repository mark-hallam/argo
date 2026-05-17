//! Operator-facing rating + flag subcommand args.
//!
//! Extracted from `cli_subcommands_skill.rs` under the
//! `codex-split-files-450-500-wave` brief to keep the parent below
//! the 420-LOC headroom target. Variants stay tuple-shaped
//! (`Rate(SkillRateCmd)`) so handler dispatch destructures the
//! struct fields directly — no behavior change.
use clap::Args;

#[derive(Args, Debug)]
pub struct SkillRateCmd {
    /// Active local skill name to rate.
    pub name: String,
    /// Skill version label being rated. Defaults to the active
    /// version of `name` if omitted.
    #[arg(long)]
    pub version: Option<String>,
    /// `up` or `down`.
    #[arg(long)]
    pub vote: String,
    /// Optional free-form comment (truncated at 500 chars).
    #[arg(long)]
    pub comment: Option<String>,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillRatingsCmd {
    /// Skill name.
    pub name: String,
    /// Skill version. Defaults to the active version.
    #[arg(long)]
    pub version: Option<String>,
    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillSetFlagCmd {
    /// Skill name to flag.
    pub name: String,
    /// Set `no-forge-feedback=1` for this skill so the Forge
    /// ingestion path skips its operator votes.
    #[arg(long, conflicts_with = "clear_no_forge_feedback")]
    pub no_forge_feedback: bool,
    /// Clear `no-forge-feedback` for this skill (set value=0).
    #[arg(long = "clear-no-forge-feedback")]
    pub clear_no_forge_feedback: bool,
    /// Emit a JSON envelope `{skill, flag, value}` instead of
    /// the human-readable confirmation line.
    #[arg(long)]
    pub json: bool,
}
