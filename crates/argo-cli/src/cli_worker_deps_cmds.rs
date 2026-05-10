//! `argo worker deps` subcommand schema (Stream
//! `claude-worker-dependency-tracking`, V1.1). Lives in a sibling file
//! so `cli_worker_cmds.rs` stays under the 500-LOC per-file ceiling —
//! same pattern used by `cli_worker_okr_cmds.rs`.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerDepsCmd {
    /// Declare that worker `<a>` depends on worker `<b>` for relation
    /// `<kind>`. Same-kind cycles are rejected; cross-kind cycles are
    /// permitted. Idempotent on `(a, b, kind)`.
    Add {
        /// The dependent (`<a>` depends on `<b>`).
        a: String,
        /// The dependency (`<a>` depends on `<b>`).
        b: String,
        /// One of `data|approval|handoff|skill`.
        #[arg(long)]
        kind: String,
        /// Optional operator note recorded with the row.
        #[arg(long)]
        note: Option<String>,
        /// Emit JSON instead of a human-readable confirmation.
        #[arg(long)]
        json: bool,
    },
    /// Remove the declared dependency `<a> → <b>` for one `<kind>`.
    Remove {
        /// The dependent.
        a: String,
        /// The dependency.
        b: String,
        /// One of `data|approval|handoff|skill`.
        #[arg(long)]
        kind: String,
        /// Emit JSON instead of a human-readable confirmation.
        #[arg(long)]
        json: bool,
    },
    /// List dependencies touching one worker.
    List {
        /// Worker id to query.
        worker_id: String,
        /// `out` (default) — what this worker depends on.
        /// `in` — what depends on this worker.
        /// `both` — union of the two.
        #[arg(long, default_value = "out")]
        direction: String,
        /// Output format: `markdown` (default) or `json`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Dump the entire workspace dependency graph.
    Graph {
        /// Output format: `markdown` (default), `dot`, or `json`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Transitive closure of workers downstream-impacted by `<id>`,
    /// sorted by depth.
    Impact {
        /// Worker id whose downstream impact to compute.
        worker_id: String,
        /// Emit JSON instead of a human-readable list.
        #[arg(long)]
        json: bool,
    },
}
