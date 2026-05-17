//! `argo skill catalog` + `argo skill import` subcommand args.
//!
//! Extracted from `cli_subcommands_skill.rs` under the
//! `codex-split-files-450-500-wave` brief. Both subcommands carry
//! enough flags that inlining them in `SkillCmd` pushed the parent
//! over the 420-LOC target. Variants are tuple-shaped
//! (`Catalog(SkillCatalogCmd)`) and the handler destructures fields
//! at dispatch — no behavior change.
use clap::Args;

#[derive(Args, Debug)]
pub struct SkillCatalogCmd {
    /// Filter to marketplace entries whose tenant (vertical
    /// analog) matches this id (case-insensitive). Unknown
    /// values surface a `vertical <id> not found` error.
    #[arg(long)]
    pub vertical: Option<String>,
    /// Substring match (case-insensitive) against the
    /// marketplace entry description.
    #[arg(long)]
    pub tag: Option<String>,
    /// Show only locally-installed marketplace entries.
    #[arg(long, conflicts_with = "not_installed")]
    pub installed_only: bool,
    /// Show only entries that are NOT locally installed.
    #[arg(long)]
    pub not_installed: bool,
    /// Explicit no-filter flag. Present for parity with the
    /// brief; the default behaviour is already to show all
    /// entries.
    #[arg(long)]
    pub all: bool,
    /// Emit a JSON array `[{ id, name, vertical, tags,
    /// installed, version, installed_version? }]` instead of
    /// the human-readable table.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillImportCmd {
    /// Artifact path.
    pub path: String,
    /// Rename on import; required to avoid duplicate-name
    /// conflicts when the original name is already installed.
    #[arg(long)]
    pub name: Option<String>,
    /// Resolve the target skill store as if running from this
    /// workspace path.
    #[arg(long)]
    pub workspace: Option<String>,
    /// Validate and print what would be installed without
    /// writing the store.
    #[arg(long)]
    pub dry_run: bool,
    /// Emit JSON.
    #[arg(long)]
    pub json: bool,
}
