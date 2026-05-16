//! `argo skill pack` — operator-facing vertical-template pack ops.
//! Today this is just the `lint` subcommand
//! (`mission-skill-pack-lint-cli`); future surfaces (verify,
//! bundle, sign) extend this enum so the long-tail `SkillCmd`
//! enum stays out of the 500-LOC headroom.

use clap::Subcommand;

/// `argo skill pack <sub>` tree.
#[derive(Subcommand, Debug)]
pub enum SkillPackCmd {
    /// Lint a vertical-template pack directory before publish —
    /// validates `pack.toml`, Skill IRs, memory schema, eval
    /// fixtures, naming, semver, and the 500-LOC file limit.
    Lint {
        /// Pack root directory to lint.
        path: String,
        /// Treat warnings as errors (exit code 2).
        #[arg(long)]
        strict: bool,
        /// Emit machine-readable JSON instead of the
        /// text-formatted finding list.
        #[arg(long)]
        json: bool,
    },
}
