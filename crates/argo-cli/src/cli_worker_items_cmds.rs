//! `argo worker items` subcommands. Extracted from
//! `cli_worker_cmds.rs` so that file stays under the 500-LOC
//! per-file ceiling (`claude-cli-worker-cmds-split`).
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerItemsCmd {
    /// List every `item_id` the worker has ever touched in the
    /// action ledger, with per-item rollups: sold / active state,
    /// total actions, count per action-state, latest activity.
    /// Operator's "what's in inventory?" view.
    List {
        /// Filter to a single vertical's namespace.
        #[arg(long)]
        vertical: Option<String>,
        /// Filter by sold status. `sold` shows items with a
        /// completed `mark_item_sold` row. `active` shows items
        /// with no completed mark_item_sold (i.e. still in
        /// inventory). `all` (default) shows both.
        #[arg(long, default_value = "all")]
        state: String,
        /// Cap the number of rows printed. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit JSON-lines (one row per item) instead of a
        /// human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Show the full action history for one item, in time order.
    /// Unlike `actions list --item-id <id>` (which shows the
    /// latest snapshot per action), `items show` walks every
    /// `worker_action` memory row tagged with the given
    /// `item_id` so the operator can audit who approved what,
    /// when it executed, and what the result was. Designed for
    /// "why is skates-A still in inventory?" forensics.
    Show {
        /// The `item_id` to inspect. Exact-match against the
        /// `item_id` field stamped at emission time.
        item_id: String,
        /// Filter to a single vertical's namespace. When the
        /// same item_id exists under multiple verticals (rare
        /// but possible), required to disambiguate.
        #[arg(long)]
        vertical: Option<String>,
        /// Emit JSON (full row dump) instead of the human-
        /// readable timeline. JSON output is one object per row
        /// in chronological order.
        #[arg(long)]
        json: bool,
    },
}
