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
    /// List the built-in Genesis archetypes operators can start from.
    ListArchetypes {
        /// Emit a single JSON object instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Show one archetype's action vocabulary, persona traits, and channels.
    ShowArchetype {
        /// Archetype label or slug, e.g. `Marketing Intern` or `marketing_intern`.
        name: String,
        /// Emit a single JSON object instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Preview the top archetype matches for a charter without writing files.
    Suggest {
        /// Plain-English role charter to classify.
        #[arg(long)]
        charter: String,
        /// Emit a single JSON object instead of a table.
        #[arg(long)]
        json: bool,
    },
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
        /// Stream `claude-genesis-llm-refinement` — opt-in LLM
        /// refinement pass on top of the deterministic pipeline.
        /// Requires `ANTHROPIC_API_KEY`. The safety judge re-runs
        /// on every action the model proposes; refinement
        /// failures fall back silently to the deterministic
        /// output with a `tracing::warn!`.
        #[arg(long)]
        refine: bool,
    },
}
