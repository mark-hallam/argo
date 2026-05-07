//! `MemoryCmd` enum — extracted from `cli_subcommands.rs` to
//! keep that file under the 500-LOC budget. Re-exported from
//! `cli_subcommands.rs` so existing `use crate::cli::MemoryCmd`
//! callers stay unchanged.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum MemoryCmd {
    /// Inspect and seed tenant-scoped shared-team memory.
    SharedTeam {
        #[command(subcommand)]
        cmd: SharedTeamCmd,
    },
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
    /// Pass `--session <uuid>` (#697) to attribute the entry to
    /// a specific session — required when the entry will later
    /// be promoted via `argo skill review promote`, since the
    /// promote audit row attributes to that session id.
    Append {
        file: String,
        body: String,
        #[arg(long)]
        vertical: Option<String>,
        #[arg(long)]
        session: Option<String>,
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
    /// Run a Dreams API summary now for one worker. Walks the
    /// previous calendar month's recent rows, calls the Dreams API,
    /// and persists the curated summary as a memory row tagged
    /// `source=dreams_api`. Refuses if the worker has any open
    /// anomaly events. Honours `ARGO_DREAMS_DAILY_CAP_USD_MICROS`.
    /// Available only when `argo-memory/dreams-backend` is enabled
    /// in the build (V2.0 Integration A).
    DreamNow {
        /// Worker id to dream for (UUID).
        worker: String,
        /// Memory scope (worker_self|role|shared_team). Defaults to
        /// `worker_self`.
        #[arg(long, default_value = "worker_self")]
        scope: String,
        /// Emit a single JSON object instead of human-readable text.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum SharedTeamCmd {
    /// List recent shared-team entries for one tenant.
    List {
        /// Tenant id (UUID).
        #[arg(long)]
        tenant: String,
        /// Maximum number of newest rows to return.
        #[arg(long)]
        limit: Option<u32>,
        /// Emit JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Write an operator-seeded shared-team entry.
    Write {
        /// Tenant id (UUID).
        #[arg(long)]
        tenant: String,
        /// Worker id credited as the author.
        #[arg(long)]
        author: String,
        /// Entry content.
        #[arg(long)]
        content: String,
        /// Emit JSON instead of a human-readable acknowledgement.
        #[arg(long)]
        json: bool,
    },
    /// Search shared-team entries with a case-insensitive regex.
    Grep {
        /// Tenant id (UUID).
        #[arg(long)]
        tenant: String,
        /// Regex pattern to match against entry content.
        #[arg(long)]
        pattern: String,
        /// Emit JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },
}
