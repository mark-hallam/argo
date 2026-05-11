//! `argo genesis …` subcommand enum (#G-genesis). Lives in its
//! own file to keep the cli enum file under the per-file
//! 500-LOC budget; re-exported via `cli.rs`'s glob.
//!
//! v1 ships one subcommand: `argo genesis create`. Future
//! additions (e.g. `argo genesis refine` for LLM-driven
//! follow-up passes) plug in here without reshaping the cli.

use clap::{ArgAction, Subcommand};
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
    /// Re-run the Genesis LLM refinement pass against an existing vertical.
    Refine {
        /// Existing vertical id discoverable via ARGO_VERTICALS_DIR or ./verticals.
        vertical_id: String,
        /// Explicitly run the safety judge after each refinement iteration.
        #[arg(long, conflicts_with = "no_judge")]
        judge: bool,
        /// Skip the safety judge re-run after refinement.
        #[arg(long = "no-judge", conflicts_with = "judge")]
        no_judge: bool,
        /// Maximum LLM refinement iterations to run.
        #[arg(long, default_value_t = 1)]
        max_iterations: usize,
        /// Print the manifest diff without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
        /// After the refinement pass, print the list of
        /// action_types newly added to the manifest's vocabulary
        /// during this run (#60 mission load-bearing wire). Empty
        /// list when no vocabulary grew.
        #[arg(long)]
        show_vocab_evolution: bool,
    },
    /// Walk the operator through a 6-prompt Q&A then generate a
    /// vertical from the answers. Stream
    /// `claude-genesis-charter-conversation`. Use
    /// `--non-interactive <yaml>` to feed pre-recorded answers
    /// (CI-friendly) and `--refine` to also run the LLM
    /// refinement pass. The final vertical lands at `--output`,
    /// the same as `argo genesis create`.
    Interactive {
        /// Output directory for the generated vertical. Genesis
        /// refuses to clobber a non-empty dir.
        #[arg(long)]
        output: PathBuf,
        /// Skip stdin prompting; load answers from a YAML file
        /// shaped like `CharterDialogue` (see
        /// `argo-genesis::charter_dialogue`).
        #[arg(long, value_name = "YAML")]
        non_interactive: Option<PathBuf>,
        /// Emit a single JSON envelope summarising the dialogue
        /// and resulting vertical instead of the human-readable
        /// post-create report.
        #[arg(long)]
        json: bool,
        /// Run the LLM refinement pass on the rendered charter.
        /// Same contract as `argo genesis create --refine`:
        /// requires `ANTHROPIC_API_KEY` and falls back silently
        /// to the deterministic plan on transient API failures.
        #[arg(long)]
        refine: bool,
    },
    /// Import an existing prompt source (LangChain `*.py`,
    /// markdown, or raw text) into a minimal vertical scaffold.
    /// Genesis revise + the action/eval/memory generators can then
    /// flesh the scaffold out. v0 reads local paths only — URL
    /// fetch needs a security review and lands in a follow-up.
    Import {
        /// Local path to the prompt source. Required.
        #[arg(long)]
        source: PathBuf,
        /// Vertical id for the new scaffold (lowercase ASCII +
        /// digits + `-`, same shape as `vertical.id`).
        #[arg(long)]
        name: String,
        /// Source format. `auto` (default) sniffs by extension:
        /// `*.py` → langchain, `*.md` → markdown, anything else →
        /// raw.
        #[arg(long, default_value = "auto")]
        format: String,
        /// Parent directory for the scaffold. The vertical lands at
        /// `<output>/<name>/`.
        #[arg(long, default_value = "verticals")]
        output: PathBuf,
        /// Emit a single JSON envelope summarising the import
        /// instead of human-readable lines.
        #[arg(long)]
        json: bool,
    },
    /// Surgically revise an already-generated vertical from operator
    /// feedback. Updates one or more of the system prompt, action
    /// vocabulary, and eval-seed list while preserving anything wrapped
    /// in `[KEEP]…[/KEEP]` markers. See
    /// `docs/build-plan/streams/claude-genesis-iteration-cycle.md`.
    Revise {
        /// Vertical id to revise (directory name under the verticals root).
        #[arg(long)]
        vertical: String,
        /// Plain-English operator feedback (`"be more terse"`,
        /// `"drop the social-media actions"`, …).
        #[arg(long)]
        feedback: String,
        /// Which artefact(s) to update: `prompt`, `actions`, `eval`,
        /// or `all` (default).
        #[arg(long, default_value = "all")]
        target: String,
        /// Print the diff and exit without writing to disk.
        #[arg(long)]
        dry_run: bool,
        /// Emit a JSON envelope instead of human-readable lines.
        #[arg(long)]
        json: bool,
    },
    /// Ingest a job description (PDF path, plain text, or http(s) URL),
    /// synthesise a charter, and run the Genesis pipeline against it.
    /// Mission `mission-genesis-job-description-ingest` (#64).
    ///
    /// Input is a single positional `<input>` interpreted in this order:
    /// 1. `http://` or `https://` prefix → URL fetch (30s timeout, 5MB cap).
    /// 2. Path that exists and ends in `.pdf` → PDF text extraction.
    /// 3. Path that exists → read as plain text.
    /// 4. Anything else → treated as a raw text body.
    FromJobDescription {
        /// Path-or-URL-or-raw-text body. See command docs above.
        input: String,
        /// Emit the full `GenesisDraft` as JSON instead of a human
        /// readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Seed Genesis from an existing sibling vertical. Operator
    /// supplies a charter file + the `--seed <vertical-id>` of an
    /// already-installed in-tenant vertical; Genesis runs the
    /// standard archetype pipeline, then merges the seed's
    /// `[actions]`, `[skills]`, `[workflows]` shape into the
    /// baseline. The seed's id is stamped onto the resulting
    /// manifest's `vertical.tags` as `"seeded-from:<id>"` so
    /// downstream eval / Forge cohort analysis can group skills
    /// by seed lineage. Mission
    /// `mission-genesis-transfer-from-tenant`.
    ///
    /// Without `--seed`, prefer `argo genesis create` — this
    /// subcommand ALWAYS seeds.
    FromVertical {
        /// Path to a plain-text file containing the role charter.
        /// Read as UTF-8; first 4 KiB is treated as the charter.
        #[arg(value_name = "CHARTER_FILE")]
        charter_file: PathBuf,
        /// Vertical id to seed from (must be discoverable under
        /// `$ARGO_VERTICALS_DIR` or `./verticals`).
        #[arg(long)]
        seed: String,
        /// Public-facing persona name. Same shape as `create`.
        #[arg(long)]
        persona: String,
        /// Output directory for the generated vertical. Genesis
        /// refuses to clobber a non-empty dir.
        #[arg(long)]
        output: PathBuf,
        /// Emit a JSON summary instead of the human-readable
        /// post-create report.
        #[arg(long)]
        json: bool,
    },
    /// Export an installed vertical as a portable tar.gz bundle.
    Export {
        /// Vertical id discoverable via ARGO_VERTICALS_DIR or ./verticals.
        #[arg(long)]
        vertical: String,
        /// Output path. Defaults to `<vertical>.argo-vertical.tar.gz`.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Include `skills/` from the vertical root when present.
        #[arg(long, default_value_t = true, action = ArgAction::Set)]
        include_skills: bool,
        /// Write only BUNDLE.toml instead of a tarball.
        #[arg(long)]
        manifest_only: bool,
        /// Overwrite an existing output file.
        #[arg(long)]
        force: bool,
    },
}
