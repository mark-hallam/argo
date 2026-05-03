//! `MemoryCmd` enum — extracted from `cli_subcommands.rs` to
//! keep that file under the 500-LOC budget. Re-exported from
//! `cli_subcommands.rs` so existing `use crate::cli::MemoryCmd`
//! callers stay unchanged.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum MemoryCmd {
    List {
        /// Output as JSON (one object per entry) for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`
        /// (renders a markdown table). `--json` wins if both are
        /// passed.
        #[arg(long, default_value = "text")]
        format: String,
        /// Limit to one file: memory|user|soul|agents.
        #[arg(long)]
        file: Option<String>,
        /// Sort order: `learned-at` (default, descending) or
        /// `size` (body byte length descending — biggest first).
        #[arg(long, default_value = "learned-at")]
        sort_by: String,
        /// Truncate to the first N rows (after sorting). Useful
        /// when the corpus is large and you only want the top-N
        /// most-recent / biggest entries.
        #[arg(long)]
        limit: Option<u32>,
        /// Filter to entries learned within this many recent
        /// minutes. Mirrors the gateway's `?since-minutes=N`.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Filter by lock status: `none` / `user-protected` /
        /// `content-locked`. Mirrors the gateway's `?lock=…`.
        #[arg(long)]
        lock: Option<String>,
        /// Case-insensitive substring filter on the entry body.
        /// Mirrors the gateway's `?body-contains=…`.
        #[arg(long)]
        body_contains: Option<String>,
        /// Scope to a vertical's namespace (#680, Rule 7).
        /// Returns ONLY entries with `vertical_id = <id>` —
        /// excludes legacy workspace-global rows. Without this
        /// flag the legacy list-all behaviour is preserved.
        #[arg(long)]
        vertical: Option<String>,
    },
    View {
        entry: String,
        /// Output the entry as a single JSON object (incl. body).
        #[arg(long)]
        json: bool,
    },
    /// Append a fresh memory entry. `<file>` is one of memory|user|soul|agents.
    /// Pass `--vertical <id>` (#680, Rule 7) to tag the entry
    /// with a vertical namespace; vertical-aware reads via
    /// `--vertical` will then surface it.
    Append {
        file: String,
        body: String,
        #[arg(long)]
        vertical: Option<String>,
    },
    /// Print just the count of active memory entries. Mirrors
    /// `argo session count`. Default: one int per line; `--json`
    /// emits `{count, file}`.
    Count {
        /// Limit to one file: memory|user|soul|agents (default
        /// counts across all four).
        #[arg(long)]
        file: Option<String>,
        /// Filter to entries learned within this many recent
        /// minutes. Mirrors the gateway's `?since-minutes=N`.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Verify a memory entry's body matches its stored SHA-256 hash.
    /// Pass `--all` to verify every active entry across all four
    /// memory files in the workspace.
    Verify {
        #[arg(required_unless_present = "all")]
        entry: Option<String>,
        /// Verify every active memory entry instead of one named entry.
        #[arg(long, conflicts_with = "entry")]
        all: bool,
    },
    Edit {
        entry: String,
    },
    Revert {
        entry: String,
    },
    Forget {
        entry: String,
    },
    Lock {
        entry: String,
    },
    Unlock {
        entry: String,
    },
}
