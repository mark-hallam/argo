//! `argo feedback ...` subcommand enum (Stream
//! `claude-operator-feedback-channel`). Lives in its own file
//! per the per-file LOC budget convention.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum FeedbackCmd {
    /// Record a thumbs-up for an action.
    Up {
        /// Action id the operator is voting on.
        action_id: String,
        /// Optional free-form comment captured alongside the vote.
        #[arg(long)]
        comment: Option<String>,
        /// Tag the vote with the worker that emitted the action.
        /// Used by `argo feedback list --worker <id>`.
        #[arg(long)]
        worker: Option<String>,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Record a thumbs-down for an action.
    Down {
        /// Action id the operator is voting on.
        action_id: String,
        /// Optional free-form comment captured alongside the vote.
        #[arg(long)]
        comment: Option<String>,
        /// Tag the vote with the worker that emitted the action.
        #[arg(long)]
        worker: Option<String>,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// List feedback rows. Optionally filter by worker and/or vote.
    List {
        /// Restrict to one worker's actions.
        #[arg(long)]
        worker: Option<String>,
        /// Restrict to one vote (`up` or `down`).
        #[arg(long)]
        vote: Option<String>,
        /// Cap the number of rows printed. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Output as a single JSON array.
        #[arg(long)]
        json: bool,
    },
}
