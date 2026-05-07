//! `argo plan ...` subcommand enum (Stream
//! `claude-runtime-planner-trait`). Lives in its own file per the
//! per-file LOC budget convention.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum PlanCmd {
    /// Decompose a coarse goal into a workflow DAG. The default
    /// uses the offline TemplatePlanner so this is hermetic.
    /// Pass `--llm` (with `ANTHROPIC_API_KEY` set) to use the
    /// LlmPlanner. The YAML goes to stdout unless `--output` is
    /// given.
    Generate {
        /// One-line goal for the planner.
        goal: String,
        /// Worker id to default tasks to. Required.
        #[arg(long)]
        worker: String,
        /// Use the LlmPlanner instead of TemplatePlanner.
        #[arg(long)]
        llm: bool,
        /// Optional vertical id forwarded to the LlmPlanner prompt.
        #[arg(long)]
        vertical: Option<String>,
        /// Path to write the YAML. Default: stdout.
        #[arg(long)]
        output: Option<String>,
        /// Output the JSON envelope (`{reasoning, workflow_yaml,
        /// planner_kind, template_name}`) instead of bare YAML.
        #[arg(long)]
        json: bool,
    },
    /// Render a previously generated YAML + reasoning sidecar.
    /// `target` may be a YAML file or a "run id" — for v1 the
    /// run id is interpreted as a path under
    /// `<workspace>/.argo/plan_runs/<id>.yaml`.
    Show {
        /// File path or run id.
        target: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Run a workflow YAML through the offline mock executor (or
    /// dry-run only with `--dry-run`). Real action emit is
    /// gated by a follow-up brief; for v1 this proves the DAG
    /// is shaped correctly.
    Execute {
        /// Workflow YAML to run.
        yaml_path: String,
        /// Validate + topo-sort only, do not run any tasks.
        #[arg(long)]
        dry_run: bool,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Manage Dreams-distilled learned plan templates (Stream
    /// `claude-dreams-plan-templates`). Cohorts of similar
    /// successful Planner decompositions get distilled by the
    /// Dreams API into reusable workflow YAML; operators promote
    /// good ones via this surface so the TemplatePlanner can
    /// match them ahead of the shipped defaults.
    Template {
        #[command(subcommand)]
        cmd: PlanTemplateCmd,
    },
}

/// `argo plan template ...` sub-subcommands.
#[derive(Subcommand, Debug)]
pub enum PlanTemplateCmd {
    /// List Dreams-distilled learned plan templates. Filter by
    /// `--grade` (`pending` / `promoted` / `rejected`).
    List {
        /// Filter by lifecycle state.
        #[arg(long)]
        grade: Option<String>,
        /// Cap on rows returned (default 50, hard ceiling 500).
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Show one learned plan template — header + workflow body.
    Show {
        /// Template id.
        id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Promote a Pending template to Promoted so the
    /// TemplatePlanner can match it.
    Promote {
        /// Template id.
        id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
}
