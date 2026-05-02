//! `argo vertical …` subcommand enum. Lives in its own file so the
//! shared `cli_subcommands.rs` stays under the 500-LOC budget. Re-
//! exported via `cli.rs`'s wildcard glob — callers don't need to
//! know the path.
//!
//! All three subcommands are read-only (#650). Install / eval /
//! run land in later PRs.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum VerticalCmd {
    /// List loaded verticals discovered from `./verticals` (or
    /// the path in `$ARGO_VERTICALS_DIR`). Prints id / name /
    /// version per row, plus any per-vertical load errors.
    List {
        /// Output as a single JSON object
        /// `{ok, loaded:[…], errors:[…]}`.
        #[arg(long)]
        json: bool,
    },
    /// Show one vertical's manifest summary by id.
    Show {
        /// The vertical's id (matches `[vertical].id` in
        /// `vertical.toml`).
        id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Re-run the cross-reference validator on one vertical and
    /// exit non-zero on any error. Mirrors what the loader does
    /// at startup; useful as a CI gate while authoring a
    /// vertical.
    Validate {
        /// The vertical's id.
        id: String,
        /// Output as a single JSON object `{id, valid, error?}`.
        #[arg(long)]
        json: bool,
    },
}
