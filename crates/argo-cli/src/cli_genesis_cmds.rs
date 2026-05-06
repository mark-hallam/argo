//! `argo genesis …` subcommand enum (#G-genesis). Lives in its
//! own file to keep the cli enum file under the per-file
//! 500-LOC budget; re-exported via `cli.rs`'s glob.
//!
//! v1 ships one subcommand: `argo genesis create`. Future
//! additions (e.g. `argo genesis refine` for LLM-driven
//! follow-up passes) plug in here without reshaping the cli.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum GenesisCmd {
    /// Generate a vertical from a plain-English charter. Runs
    /// the archetype classifier, assembles the manifest,
    /// auto-fixes sensitive actions to require approval, runs
    /// the safety judge, then writes the complete
    /// `verticals/<id>/` tree to `--output`.
    Create {
        /// Plain-English description of the role. Required.
        /// Example: `"be my marketing intern, run weekly
        /// campaigns to cold leads"`.
        #[arg(long)]
        charter: String,
        /// Public-facing persona name (`maya`, `sam`, …). Used
        /// in the worker_id + the persona prompt's signoff.
        /// Must contain only `[a-zA-Z0-9_-]`.
        #[arg(long)]
        persona: String,
        /// Output directory. Genesis refuses to clobber a
        /// non-empty dir — use a fresh path.
        #[arg(long)]
        output: PathBuf,
        /// Emit a JSON summary instead of the human-readable
        /// post-create report.
        #[arg(long)]
        json: bool,
    },
}
