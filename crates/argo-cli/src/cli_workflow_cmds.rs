//! `argo workflow ...` subcommand enum (Stream
//! `claude-workflow-dag`). Lives in its own file so
//! `cli_subcommands.rs` stays under the per-file LOC budget.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkflowCmd {
    /// Validate a workflow YAML file: parses, checks the DAG
    /// topology (no cycles, every edge endpoint declared), and
    /// reports the topological execution order.
    Validate {
        /// Path to a workflow YAML file.
        path: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Run a workflow. Without `--dry-run`, executes every task
    /// against the [mock] runner — production worker dispatch
    /// wiring is a follow-up. With `--dry-run`, prints the
    /// topological execution plan without invoking the runner.
    Run {
        /// Path to a workflow YAML file.
        path: String,
        /// Print the execution plan without invoking the runner.
        #[arg(long)]
        dry_run: bool,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Show the latest report for one workflow run id.
    Status {
        /// Workflow run id (UUIDv7) returned by `run`.
        run_id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// List workflow runs in this workspace.
    ListRuns {
        /// Cap the number of rows printed. Default 50.
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Resume a previously-checkpointed workflow run. Replays
    /// `workflow_checkpoint` rows to restore prior outputs and
    /// runs only the still-pending tasks.
    Resume {
        /// Workflow run id (UUIDv7) returned by `run`.
        run_id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// List every checkpoint persisted for one workflow run, in
    /// seq order.
    Checkpoints {
        /// Workflow run id.
        run_id: String,
        /// Output as a single JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Drop every checkpoint row strictly above `--to-checkpoint`
    /// for one run. Subsequent `argo workflow resume` will pick
    /// up from the rolled-back point.
    Rollback {
        /// Workflow run id.
        run_id: String,
        /// Target seq to roll back to (rows with seq > target are dropped).
        #[arg(long = "to-checkpoint")]
        to_checkpoint: i64,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
}
