//! Publish-side skill subcommand argument structs.
//!
//! Extracted from `cli_subcommands_skill.rs` under the
//! `claude-cli-subcommands-skill-split` brief (parent file was at
//! 596 LOC — over the 500-LOC HARD RULE 6 ceiling). The three
//! structs live here verbatim; the parent file re-exports them so
//! existing handler call sites continue to import via the original
//! path.
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct SkillPublishCmd {
    /// Local active skill name to publish.
    pub name: String,
    /// Registry reference. `mock://path` writes to a filesystem registry.
    #[arg(long, default_value = "mock://.argo/skill-registry")]
    pub registry: String,
    /// Marketplace tag. Defaults to the source skill version.
    #[arg(long)]
    pub tag: Option<String>,
    /// Read a SKILL.md file instead of the active local skill row.
    #[arg(long)]
    pub manifest: Option<PathBuf>,
    /// Print the bundle plan without writing files or calling a registry.
    #[arg(long)]
    pub dry_run: bool,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillUnpublishCmd {
    /// Marketplace reference in `<name>:<tag>` form.
    pub reference: String,
    /// Registry reference. `mock://path` reads a filesystem registry.
    #[arg(long, default_value = "mock://.argo/skill-registry")]
    pub registry: String,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SkillTagsCmd {
    /// Skill name whose marketplace tags should be listed.
    pub name: String,
    /// Registry reference. `mock://path` reads a filesystem registry.
    #[arg(long, default_value = "mock://.argo/skill-registry")]
    pub registry: String,
    /// Maximum number of tags to list.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}
