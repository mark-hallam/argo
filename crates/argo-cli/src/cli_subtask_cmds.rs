//! `argo subtask ...` subcommand enum
//! (Stream `claude-subtask-ledger-store`).
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SubtaskCmd {
    /// List subtask rows. Filter by `--session`, `--worker`,
    /// `--parent-goal`, or `--state`. Newest-first, capped at
    /// `--limit` (default 50).
    List {
        /// Restrict to one parent session id.
        #[arg(long)]
        session: Option<String>,
        /// Restrict to one worker id.
        #[arg(long)]
        worker: Option<String>,
        /// Restrict to an exact parent goal string.
        #[arg(long = "parent-goal")]
        parent_goal: Option<String>,
        /// Restrict to one lifecycle state: `pending`, `running`,
        /// `done`, `failed`, `skipped`.
        #[arg(long)]
        state: Option<String>,
        /// Cap on returned rows. Default 50, hard cap 500.
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Output the JSON envelope (`{rows, count, filters}`)
        /// instead of a text table.
        #[arg(long)]
        json: bool,
    },
    /// Show one row in full.
    Show {
        /// Subtask id (UUID).
        id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Render the subtask tree for one parent session id, oldest
    /// row first. Useful for "what plan did the worker generate
    /// for session X?".
    Tree {
        /// Parent session id.
        session: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Render the per-delegation budget tree rooted at a subtask
    /// id (Stream `mission-delegation-budget-tree`). Aggregated
    /// spend rolls up to each node via the recursive walk_tree
    /// CTE; declared budget is taken from each row's
    /// `budget_usd_cents` (NULL = unbounded).
    Budget {
        /// Root subtask id whose descendants to walk.
        root_id: String,
        /// Output as a single JSON object instead of a text tree.
        #[arg(long)]
        json: bool,
    },
}
