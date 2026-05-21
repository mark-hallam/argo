//! `argo workflow def ...` subcommand enum (Stream
//! `mission-pm-workflow-rollback-cli`).
//!
//! Lives in its own file so `cli_workflow_cmds.rs` stays under the
//! per-file LOC budget. The six variants below mirror the brief's
//! `list / show / versions / rollback / diff / archive` surface; the
//! `Def` parent prefix in [`crate::cli::WorkflowCmd`] keeps these
//! semantically distinct from the DAG-run rollback already exposed
//! by `argo workflow rollback <run-id>`.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkflowDefCmd {
    /// `argo workflow def list <vertical>` — list every workflow
    /// definition with an active head pointer under `vertical`. Skips
    /// drafts and archived versions without a head.
    List {
        /// Vertical id (e.g. `pr_checker`).
        vertical: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// `argo workflow def show <vertical> <name> [--version <id>]` —
    /// print the TOML body. Defaults to the current head when
    /// `--version` is absent. Body goes to stdout so it pipes cleanly
    /// to `> file.toml`.
    Show {
        /// Vertical id.
        vertical: String,
        /// Workflow name.
        name: String,
        /// Optional version id; defaults to the head version.
        #[arg(long)]
        version: Option<String>,
        /// Output as a single JSON object (default is bare TOML).
        #[arg(long)]
        json: bool,
    },
    /// `argo workflow def versions <vertical> <name>` — list every
    /// version row for `(vertical, name)`, newest first.
    Versions {
        /// Vertical id.
        vertical: String,
        /// Workflow name.
        name: String,
        /// Output as a single JSON array.
        #[arg(long)]
        json: bool,
    },
    /// `argo workflow def rollback <vertical> <name> --to <version-id>` —
    /// set the head pointer for `(vertical, name)` to `--to`. Requires
    /// `--yes` for non-interactive use; otherwise prompts with `y/N`.
    Rollback {
        /// Vertical id.
        vertical: String,
        /// Workflow name.
        name: String,
        /// Target version id (must already exist under the same
        /// `(tenant, vertical, name)` triple).
        #[arg(long = "to")]
        to: String,
        /// Skip the interactive confirmation prompt.
        #[arg(long)]
        yes: bool,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// `argo workflow def diff <vertical> <name> <version-a>
    /// <version-b>` — emit a unified diff between two versions'
    /// TOML bodies. JSON form returns a structured envelope.
    Diff {
        /// Vertical id.
        vertical: String,
        /// Workflow name.
        name: String,
        /// Left-hand version id.
        version_a: String,
        /// Right-hand version id.
        version_b: String,
        /// Output as a structured JSON object (added/removed/changed).
        #[arg(long)]
        json: bool,
    },
    /// `argo workflow def archive <vertical> <name> --version <id>` —
    /// flip the named version's `lifecycle_state` to `archived` and
    /// clear the head pointer if the row was currently active.
    /// Requires `--yes` for non-interactive use.
    Archive {
        /// Vertical id.
        vertical: String,
        /// Workflow name.
        name: String,
        /// Version id to archive.
        #[arg(long)]
        version: String,
        /// Skip the interactive confirmation prompt.
        #[arg(long)]
        yes: bool,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
}
