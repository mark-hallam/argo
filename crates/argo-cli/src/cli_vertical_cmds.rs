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
    /// Walk every `[evaluation].suites` JSONL file declared in
    /// the manifest, validate each fixture row is well-formed,
    /// and tabulate fixture counts per suite. Useful as a CI
    /// gate that the suite hasn't drifted into bad shape (#656).
    /// Live LLM-driven scoring is deferred — that's its own
    /// PR scoped to scoring rubrics + API key handling.
    Eval {
        /// The vertical's id.
        id: String,
        /// Output as a single JSON object
        /// `{id, suites:[{path, fixture_count, errors:[…]}]}`.
        #[arg(long)]
        json: bool,
    },
    /// Run a named workflow declared by a vertical (#659). Walks
    /// the workflow's steps in declaration order. Today only
    /// `tool_call` steps run end-to-end — `skill_invoke` and
    /// `prompt` need an LLM transport and are recorded with
    /// `status="deferred_no_llm"` so the operator can see what
    /// would have run. Capability + per-vertical deny lists are
    /// enforced. Pass `--session <id>` (#668) to bind the
    /// workflow's tool calls to an existing session's audit
    /// chain so `argo audit show --session <id>` shows the
    /// activity alongside agent turns.
    Workflow {
        /// The vertical's id (`[vertical].id` in `vertical.toml`).
        vertical_id: String,
        /// The workflow's name (`workflow.name` field of the
        /// matching file in `[workflows].files`).
        workflow_name: String,
        /// Bind the workflow's tool calls to this session's
        /// audit chain (#668). Same audit shape as the agent
        /// runner so the chain verifier accepts them.
        #[arg(long)]
        session: Option<String>,
        /// Output as a single JSON object
        /// `{vertical_id, workflow, ok, total_ms, steps:[…]}`.
        #[arg(long)]
        json: bool,
    },
    /// Install a vertical from a source directory into the
    /// discovery dir (#661). Validates the source manifest first,
    /// then copies the whole tree to `<target>/<vertical-id>/`.
    /// Refuses to overwrite an existing vertical unless `--force`
    /// is passed.
    Install {
        /// Path to the source directory containing `vertical.toml`.
        source: String,
        /// Target verticals dir. Defaults to
        /// `argo_vertical::default_dir()` (i.e., `./verticals`
        /// or `$ARGO_VERTICALS_DIR`).
        #[arg(long)]
        target: Option<String>,
        /// Replace an existing installation of the same id.
        #[arg(long)]
        force: bool,
        /// Output as a single JSON object
        /// `{ok, id, name, version, root}`.
        #[arg(long)]
        json: bool,
    },
    /// Remove an installed vertical from the discovery dir
    /// (#661). Refuses by default unless `--yes` is passed; this
    /// is a destructive on-disk operation so the confirmation is
    /// the safety boundary.
    Uninstall {
        /// The vertical's id.
        id: String,
        /// Target verticals dir. Defaults to
        /// `argo_vertical::default_dir()`.
        #[arg(long)]
        target: Option<String>,
        /// Confirm the deletion. Without this flag the command
        /// prints what would happen and exits non-zero.
        #[arg(long)]
        yes: bool,
        /// Output as a single JSON object `{ok, id, removed_path}`.
        #[arg(long)]
        json: bool,
    },
    /// Run best-practice hygiene checks against a vertical
    /// beyond `validate`'s structural rules (#671). Catches
    /// orphan escalation entries, typo'd tool names against the
    /// runtime's built-in set, empty bundles / suites, and
    /// other things the operator probably didn't mean.
    Lint {
        /// The vertical's id.
        id: String,
        /// Output as a single JSON object
        /// `{ok, id, findings:[{severity, code, message}]}`.
        #[arg(long)]
        json: bool,
    },
    /// Composite "is this vertical OK?" health check (#673):
    /// validate + eval + lint in one report. Exits non-zero if
    /// any check fails. Useful as a single CI gate.
    Doctor {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Pack an installed vertical into a tar archive (#675).
    /// Inverse of `install` from a tarball — gives operators a
    /// single artifact they can ship to CI / share with a team /
    /// stash before editing.
    Pack {
        /// The vertical's id.
        id: String,
        /// Output path; defaults to `<id>.tar` in the cwd.
        #[arg(long)]
        out: Option<String>,
        /// Source verticals dir. Defaults to the discovery dir.
        #[arg(long)]
        target: Option<String>,
        /// Output as a single JSON object `{ok, id, path, bytes}`.
        #[arg(long)]
        json: bool,
    },
    /// One-shot "from zero to running vertical" composite (#669).
    /// Creates a fresh session bound to the vertical, attaches
    /// every skill the manifest declares, then runs the named
    /// `--workflow` against that session so the workflow's tool
    /// calls land in the session's audit chain. Emits one
    /// combined JSON document with both the session metadata
    /// and the workflow report.
    Run {
        /// The vertical's id.
        vertical_id: String,
        /// The workflow to run inside the freshly-created
        /// session.
        #[arg(long)]
        workflow: String,
        /// Optional human-readable session title.
        #[arg(long)]
        title: Option<String>,
        /// Provider id; defaults to `mock` so the command
        /// works without paid credentials in CI / demo flows.
        #[arg(long, default_value = "mock")]
        provider: String,
        /// Model id; defaults to `mock`.
        #[arg(long, default_value = "mock")]
        model: String,
        /// Output as a single combined JSON document.
        #[arg(long)]
        json: bool,
    },
}
