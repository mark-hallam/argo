//! Skill-version-migration subcommand argument structs.
//!
//! Extracted as a continuation of the second-pass cohesive
//! cluster under the `claude-cli-subcommands-skill-split` brief
//! (`Stop when cli_subcommands_skill.rs ≤ 480 LOC`). The three
//! `Migrate` / `Upgrade` / `ListMigrations` variants converted
//! from inline `Variant { fields }` form to `Variant(Cmd)` tuple
//! form, matching the established pattern used by `Search` and
//! `Sign`.
use clap::Args;

#[derive(Args, Debug)]
pub struct SkillMigrateCmd {
    /// Skill id (32-char hex from `argo skill versions --json`,
    /// or dashed UUID).
    pub skill_id: String,
    /// Version the migration applies FROM.
    #[arg(long)]
    pub from: String,
    /// Version the migration upgrades TO.
    #[arg(long)]
    pub to: String,
    /// Mark the migration as auto-applicable: tenants running
    /// `argo skill upgrade <name>` WITHOUT `--yes` will still
    /// apply this row.
    #[arg(long)]
    pub auto_upgrade: bool,
    /// Free-text rationale (truncated on insert).
    #[arg(long)]
    pub reason: Option<String>,
    /// Publisher tenant id (UUID). Defaults to a fresh per-call
    /// id when omitted; publisher-ownership enforcement is a
    /// separate brief.
    #[arg(long)]
    pub publisher_tenant: Option<String>,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillUpgradeCmd {
    /// Skill name (the active version's `from_version` is the
    /// lookup key).
    pub name: String,
    /// Print what would change without writing.
    #[arg(long)]
    pub dry_run: bool,
    /// Apply even when the migration row's
    /// `auto_upgrade=false`.
    #[arg(long)]
    pub yes: bool,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillListMigrationsCmd {
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}
