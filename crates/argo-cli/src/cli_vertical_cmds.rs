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
        /// Optional tenant id to echo in discovery output. Local
        /// vertical installation is workspace-scoped today; the
        /// tenant flag is carried for operator scripts that pair
        /// this with tenant-scoped worker/session commands.
        #[arg(long)]
        tenant: Option<String>,
        /// Discovery source: workspace, library, or both.
        #[arg(long, value_parser = ["workspace", "library", "both"], default_value = "both")]
        source: String,
        /// Include soft-archived verticals in the listing.
        #[arg(long)]
        include_archived: bool,
        /// Output as a single JSON object
        /// `{ok, loaded:[…], errors:[…]}`.
        #[arg(long)]
        json: bool,
    },
    /// Soft-delete a vertical so it no longer appears in default
    /// listings and cannot bind new sessions or workers.
    Archive {
        /// The vertical's id.
        id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Re-activate a previously archived vertical.
    Unarchive {
        /// The vertical's id.
        id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Permanently remove an archived vertical from disk.
    Delete {
        /// The vertical's id.
        id: String,
        /// Confirm permanent deletion. The vertical must already
        /// be archived so existing audit/session rows remain the
        /// record of prior work.
        #[arg(long)]
        force: bool,
        /// Output as a single JSON object.
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
    /// Render a one-paragraph summary of an installed vertical.
    Describe {
        /// The vertical's id (matches `[vertical].id` in
        /// `vertical.toml`).
        id: String,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
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
    ///
    /// Default mode: structural validation only (fixture counts +
    /// row-shape errors). Pass `--score` to switch to deterministic
    /// rule-based scoring (#693) — each fixture's `expected` block
    /// (must_include / must_not_include / abstain / hallucination /
    /// pattern_detected / skill_proposal / external_mutation /
    /// must_refuse_or_defer / personalization_quality) is checked
    /// against an `actual` field on the same row. Rows without an
    /// `actual` are reported as `no_actual` and don't count toward
    /// pass/fail. No LLM dependency; an LLM-judge layer can be
    /// added later as an opt-in extension.
    Eval {
        /// The vertical's id.
        id: String,
        /// Output as a single JSON object. Structural mode shape:
        /// `{id, suites:[{path, fixture_count, errors:[…]}]}`.
        /// Scored-mode shape:
        /// `{id, mode:"scored", suites:[{path, fixture_count,
        /// scored_count, no_actual, malformed, passed, failed,
        /// pass_rate, unknown_actual_ids, fixtures:[…]}]}`.
        #[arg(long)]
        json: bool,
        /// Switch to scored mode (#693). Off by default so CI
        /// gates that already depend on the structural shape stay
        /// stable.
        #[arg(long)]
        score: bool,
        /// Path to an actuals JSONL file (#694). Each row is
        /// `{"id": "<fixture_id>", "actual"|"response": "..."}`.
        /// The scorer joins by `id`; external actuals override
        /// any `actual` embedded in the canonical fixture row,
        /// so the canonical eval suite stays untouched. Ignored
        /// without `--score`. Errors loudly on duplicate ids,
        /// missing fields, or invalid JSON; warns on unknown
        /// ids that don't match any fixture.
        #[arg(long)]
        actuals: Option<String>,
        /// Optional judge layer (#698). Off by default — without
        /// this flag, scoring is purely rule-based. Two modes:
        /// `--judge heuristic` aggregates the rule check
        /// outcomes into one verdict per fixture (no LLM, no
        /// API key); `--judge <provider>/<model>` is the seam
        /// for a live LLM judge — the wiring lands in PR #698b
        /// (today the verdict carries a `not_yet_wired` reason
        /// that mirrors the rule-engine outcome). Ignored
        /// without `--score`. Reject empty / malformed specs.
        #[arg(long)]
        judge: Option<String>,
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
    /// Publish an installed vertical to an OCI registry as a
    /// versioned artifact (Stream
    /// `claude-marketplace-vertical-publish`). Packs every file
    /// under `verticals/<id>/` into a deterministic tar via
    /// `argo_marketplace::build_vertical_bundle`, then pushes to
    /// the registry. The deterministic tar means re-publishing the
    /// same on-disk state yields the same digest, so receivers can
    /// verify pulls byte-for-byte.
    Publish {
        /// The vertical's id.
        id: String,
        /// OCI registry base URL (e.g. `https://ghcr.io` or
        /// `http://localhost:5000`). When omitted, uses the
        /// in-memory mock path for backward compatibility.
        #[arg(long)]
        registry: Option<String>,
        /// Tag to publish under. Defaults to the vertical's
        /// `[vertical].version` when omitted.
        #[arg(long)]
        tag: Option<String>,
        /// Optional org/namespace prefix on the registry side
        /// (e.g. `argo-agents` → `<host>/argo-agents/<id>:<tag>`).
        /// When omitted, the bare vertical id is used as the repo.
        #[arg(long)]
        repo_org: Option<String>,
        /// Optional registry token. Takes precedence over
        /// credentials.toml and Docker config.
        #[arg(long = "token", alias = "auth-token")]
        auth_token: Option<String>,
        /// Output as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    /// Pull a vertical from an OCI registry into the local
    /// discovery dir (Stream
    /// `claude-marketplace-vertical-publish`). Verifies the tar's
    /// sha256 against the manifest's declared digest BEFORE writing
    /// any file — refuses to extract tampered bundles. Refuses if
    /// a vertical with the same id already exists locally; pass
    /// `--rename <new-id>` to pull under a different local id.
    Pull {
        /// OCI reference, e.g.
        /// `ghcr.io/argo-agents/marketplace-seller:1.0.0`.
        reference: String,
        /// Override the registry HTTPS base. When omitted, derived
        /// from the reference (HTTP for `localhost`/`127.0.0.1`,
        /// HTTPS otherwise).
        #[arg(long)]
        registry: Option<String>,
        /// Pull under a different local id when one already exists.
        #[arg(long)]
        rename: Option<String>,
        /// Optional registry token. Takes precedence over
        /// credentials.toml and Docker config.
        #[arg(long = "token", alias = "auth-token")]
        auth_token: Option<String>,
        /// Output as a single JSON object.
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
