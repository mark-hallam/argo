//! `argo skill review` subcommand enum (#684). Lives in its
//! own file so `cli_subcommands.rs` stays under the 500-LOC
//! budget. Re-exported via `cli_subcommands.rs` so callers see
//! `SkillReviewCmd` through the existing `use crate::cli::*`
//! glob.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SkillReviewCmd {
    /// List skills awaiting human review (lifecycle_state =
    /// pending_review). Default text mode prints id/name/
    /// vertical/learned-in/created-at; `--json` emits a
    /// machine-readable envelope.
    List {
        /// Truncate both persisted pending skills and memory-backed
        /// candidates after filtering.
        #[arg(long)]
        limit: Option<usize>,
        /// Restrict the review queue to one vertical id.
        #[arg(long)]
        vertical: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show one pending-review skill's body, trajectory,
    /// provenance, and originating session.
    Show {
        /// The skill's id (UUID v7).
        skill_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Approve a pending-review skill — flips lifecycle_state
    /// to active. The next agent turn that consults the skill
    /// lookup will see it.
    Approve {
        /// The skill's id (UUID v7).
        skill_id: String,
        /// Optional reviewer note recorded in the audit row.
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Reject a pending-review skill — flips lifecycle_state
    /// to rejected. The skill row stays in the table for audit
    /// but the lookup never returns it again.
    Reject {
        /// The skill's id.
        skill_id: String,
        /// Optional reviewer note recorded in the audit row.
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Promote a memory-backed skill candidate into a real
    /// `pending_review` skill row (#697). Reads the candidate
    /// from `MEMORY.md` (vertical-namespaced; isolation
    /// preserved), constructs a SKILL.md from its fields, and
    /// installs it via `propose_for_review`. The skill remains
    /// `pending_review` until the operator runs
    /// `argo skill review approve <new-skill-id>` — promote
    /// never auto-activates. Emits a `skill.candidate.promoted`
    /// audit row.
    Promote {
        /// The memory entry's UUID. Use `argo skill review
        /// list --json` to discover candidate ids.
        memory_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Export a stored skill to an offline distribution file.
    Export {
        /// The skill's id (UUID v7).
        skill_id: String,
        /// Destination file path.
        #[arg(long)]
        output: String,
        /// Output format: md (default), toml, or json.
        #[arg(long, default_value = "md")]
        format: String,
    },
    /// Import a skill distribution file into the local skill store.
    Import {
        /// Path to a md, toml, or json skill distribution.
        path: String,
        /// Optional lock override: none, content_locked, or user_protected.
        #[arg(long)]
        lock: Option<String>,
    },
}
