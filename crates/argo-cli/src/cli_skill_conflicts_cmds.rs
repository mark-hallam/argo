//! `argo skill conflicts ...` subcommand enum (Stream
//! `claude-skill-conflict-resolution`). Lives in its own file so
//! `cli_skill_review_cmds.rs` and `cli_subcommands.rs` stay
//! within their LOC budgets.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SkillConflictsCmd {
    /// List every workspace ↔ library skill-name collision.
    List {
        /// Output as a JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Show the unified diff between the two layers' bodies for
    /// one collision.
    Show {
        /// The colliding skill name.
        name: String,
        /// Output as a single JSON object (`{name, workspace,
        /// library, diff}`).
        #[arg(long)]
        json: bool,
    },
    /// Resolve a collision by archiving the loser. `--keep
    /// workspace` archives the library row; `--keep library`
    /// archives the workspace row.
    Resolve {
        /// The colliding skill name.
        name: String,
        /// Which layer to keep — `workspace` or `library`.
        #[arg(long)]
        keep: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
}
