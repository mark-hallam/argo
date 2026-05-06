//! `argo classify ...` subcommand enum (Stream
//! `claude-data-classification`). Lives in its own file per the
//! per-file LOC budget convention.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ClassifyCmd {
    /// Tag a record with a data class. The target is given as
    /// `<kind>:<id>` (e.g. `memory_entry:01964…`,
    /// `audit_record:42`). Class must be one of `pii`, `phi`,
    /// `financial`, `secret`, `public`, `untagged`.
    Set {
        /// `<record_kind>:<record_id>` — e.g. `memory_entry:abc`.
        target: String,
        /// One of `pii`, `phi`, `financial`, `secret`, `public`,
        /// `untagged`.
        class: String,
        /// Optional actor that owns this tag (worker id, operator
        /// handle, automation rule). Stored in `tagged_by`.
        #[arg(long)]
        by: Option<String>,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// List recorded tags. All filters are optional.
    List {
        /// Restrict to one class (`pii` / `phi` / ...).
        #[arg(long)]
        class: Option<String>,
        /// Restrict to one record kind (e.g. `memory_entry`).
        #[arg(long)]
        kind: Option<String>,
        /// Restrict to one author / `tagged_by` value.
        #[arg(long)]
        worker: Option<String>,
        /// Show only tags whose retention window has expired (older
        /// than the per-class `retention_days`). Useful before
        /// running a prune.
        #[arg(long)]
        expired: bool,
        /// Cap the number of rows printed. Default 100.
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Output as a single JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Print the configured per-class handling rules
    /// (`retention_days`, `redact_in_export`). Reads
    /// `<workspace>/.argo/classification.toml` if present and
    /// merges over shipped defaults.
    Rules {
        /// Output as a single JSON object keyed by class.
        #[arg(long)]
        json: bool,
    },
}
