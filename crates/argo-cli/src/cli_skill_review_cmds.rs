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
    /// vertical/learned-in/created-at; `--json` emits one
    /// object per row.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Approve a pending-review skill — flips lifecycle_state
    /// to active. The next agent turn that consults the skill
    /// lookup will see it.
    Approve {
        /// The skill's id (UUID v7).
        skill_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Reject a pending-review skill — flips lifecycle_state
    /// to rejected. The skill row stays in the table for audit
    /// but the lookup never returns it again.
    Reject {
        /// The skill's id.
        skill_id: String,
        #[arg(long)]
        json: bool,
    },
}
