//! Real-run execution path for `POST /api/v1/workers/<id>/run`.
//!
//! This is the no-theater half of the "Run now" button. The parent
//! [`super::routes_worker_run`] does HTTP plumbing (parse id, tenant
//! resolve, load worker, archived check, capability gate, audit write,
//! response shaping). This module does the actual work: validate that a
//! real turn CAN run, then run one via the proven
//! [`crate::handlers::work_dispatch::process_job`] turn-runner — the
//! same engine `argo work serve` drives — and report the TRUTH about
//! whether the turn started, completed, or failed.
//!
//! ## Design: synchronous, validate-before-claim
//!
//! - **Synchronous** (`await` one turn, then return). Chosen over an
//!   async `tokio::spawn` because the response can then carry the REAL
//!   outcome (`status: completed|failed`, `chars`, `session_id`) instead
//!   of an optimistic `running` the caller would have to poll. The SPA
//!   toast already deep-links to `/audit/action/<id>`; a synchronous
//!   result lets it also state the outcome. A "Run now" turn is capped
//!   at `MAX_STEPS = 1` / `MAX_TOOL_ROUNDS = 8`, so the request returns
//!   in one model turn — acceptable for a manual, operator-initiated
//!   click.
//! - **Validate-before-claim.** Before a single token is spent we check
//!   two preconditions, and on failure return an honest `409 Conflict`
//!   (NOT a fake success):
//!     1. the worker's `vertical_id` must load from the vertical
//!        registry — a teammate with no runnable role can't run;
//!     2. a real model provider key must be present (`ANTHROPIC_API_KEY`
//!        or `OPENAI_API_KEY`, via the canonical
//!        [`super::startup_check::provider_key_present`]).
//!   Only after both pass do we enqueue the job and run the turn, so the
//!   "started" claim is always honest.
//! - **Truthful outcome.** The turn runs through `process_job`, which
//!   writes a `completed_jobs` memory row ONLY when the turn genuinely
//!   completed. We read that row back to source `session_id`, the
//!   assistant-text length (`chars`), and the loop `stop_reason`. If no
//!   completion row exists, we report `status: failed` — we never claim
//!   a completion the engine didn't record.

use argo_identity::Worker;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde_json::Value;
use std::sync::Arc;

/// Provider used for a real "Run now" turn in production. A real
/// provider (NOT `mock`) is mandatory — the whole point of this change
/// is to do real work. The session is created with this provider/model
/// and `argo_agent_runner::run_turn` routes it via the on-disk provider
/// manifest (`manifests/providers/anthropic.toml`).
const RUN_PROVIDER: &str = "anthropic";

/// Model for a real "Run now" turn. Mirrors the Genesis LLM path's
/// pinned Anthropic model so both gateway LLM surfaces agree.
const RUN_MODEL: &str = "claude-sonnet-4-6";

/// Test-only seam: when this env var is set, it overrides the provider
/// for the run. The production gateway never sets it, so it always
/// runs the real `anthropic` provider. Tests set it to `mock` to drive
/// the FULL `process_job` engine hermetically (the mock provider
/// short-circuits the model call to a deterministic synthetic reply),
/// proving the wiring + response shape with no network. Mirrors the
/// pitch/genesis paths' provider-override discipline.
const RUN_PROVIDER_OVERRIDE_ENV: &str = "ARGO_WORKER_RUN_PROVIDER";

/// Resolve `(provider, model)` for the run — the production default
/// (`anthropic` / `claude-sonnet-4-6`) unless the test seam overrides
/// the provider. When overridden to `mock`, the model is irrelevant
/// (the mock path ignores it) but a non-empty value is still passed.
fn resolve_run_provider() -> (String, String) {
    match std::env::var(RUN_PROVIDER_OVERRIDE_ENV) {
        Ok(p) if !p.trim().is_empty() => (p.trim().to_string(), RUN_MODEL.to_string()),
        _ => (RUN_PROVIDER.to_string(), RUN_MODEL.to_string()),
    }
}

/// One step is enough for a manual "do your next task now" nudge, and
/// keeps the synchronous request bounded. `max_steps > 1` would also
/// trip the multi-step research-tools pre-flight gate inside
/// `process_job`.
const MAX_STEPS: u32 = 1;

/// Per-step ceiling on internal tool-dispatch rounds for the turn.
const MAX_TOOL_ROUNDS: usize = 8;

/// Outcome of a real "Run now" turn, surfaced to the HTTP response.
///
/// Example (crate-internal type, shown rather than doc-tested):
///
/// ```text
/// let ok = RunOutcome {
///     session_id: "abc".into(),
///     status: "completed",
///     chars: 42,
///     approved_tools: vec!["file_read".into()],
/// };
/// // ok.status == "completed"; ok.chars == 42
/// ```
#[derive(Debug)]
pub(super) struct RunOutcome {
    /// The pending-job id this run claimed (UUID string). Surfaced to
    /// the HTTP response so the SPA can deep-link the run's artifact
    /// bundle via `GET /api/v1/workers/<id>/runs/<job_id>/output`
    /// (brief `uf-spa-teammate-output-surface`). Empty on the failed
    /// path (no completed job to point at).
    pub job_id: String,
    /// The session the turn ran in (lower-hyphenated UUID string).
    pub session_id: String,
    /// `"completed"` when the engine recorded a completion row,
    /// `"failed"` otherwise. Never `"completed"` without evidence.
    pub status: &'static str,
    /// Length in characters of the assistant's reply preview the
    /// engine persisted. `0` on the failed path.
    pub chars: usize,
    /// Tools the operator pre-approved for this run that survived the
    /// worker-tool-set membership filter and were minted as ONE-SHOT
    /// session grants before the turn. Requested tools OUTSIDE the
    /// worker's declared tool set are silently absent (skipped, never
    /// granted). Empty when the request carried no `approve_tools`.
    pub approved_tools: Vec<String>,
}

/// Why a real run could not even start. Maps to an honest `409
/// Conflict` in the handler — the precondition the operator must fix,
/// never a fabricated success.
///
/// Example (crate-internal type, shown rather than doc-tested):
///
/// ```text
/// RunBlocked::NoVertical.message() == "this teammate has no runnable role yet"
/// RunBlocked::NoProvider.message() == "no model provider configured"
/// ```
#[derive(Debug)]
pub(super) enum RunBlocked {
    /// The worker's `vertical_id` does not load from the registry.
    NoVertical,
    /// No provider API key is present in the gateway environment.
    NoProvider,
}

impl RunBlocked {
    /// Stable, operator-readable reason string for the `409` body.
    pub(super) fn message(&self) -> &'static str {
        match self {
            RunBlocked::NoVertical => "this teammate has no runnable role yet",
            RunBlocked::NoProvider => "no model provider configured",
        }
    }

    /// Machine code for the error envelope's `code` field.
    pub(super) fn code(&self) -> &'static str {
        match self {
            RunBlocked::NoVertical => "no_runnable_role",
            RunBlocked::NoProvider => "no_provider_configured",
        }
    }
}

/// Validate-before-claim, then run one real turn for `worker`.
///
/// Returns `Ok(Ok(outcome))` when a turn genuinely ran (completed or
/// failed — both are honest), `Ok(Err(blocked))` when a precondition
/// failed so NO turn was claimed, and `Err(_)` only on an internal
/// store error (mapped to a 500 by the caller).
///
/// `prompt` is the operator's note when non-empty, else a generic
/// standing-task nudge — see [`super::routes_worker_run`] for how the
/// caller derives it. We never fabricate task specifics.
///
/// `approve_tools` is the operator's "Approve & run" pre-approval list
/// (keystone fix): each requested tool that passes the worker-tool-set
/// membership filter ([`filter_to_worker_tool_set`]) is minted as a
/// ONE-SHOT session grant on the new run's session BEFORE the turn, so
/// a probationary teammate's held tools clear in the same operator
/// action that triggers the run. Tools outside the worker's declared
/// tool set are skipped (never granted, never an error) — the request
/// cannot mint a capability the worker doesn't legitimately hold.
/// `approved_by` is the audit principal stamped on each grant's
/// `ApprovalGranted` row. No new auth path: the caller is the run
/// handler, which has already enforced the run capability.
pub(super) async fn run_worker_now(
    store: &Arc<SessionStore>,
    worker: &Worker,
    prompt: &str,
    approve_tools: &[String],
    approved_by: Option<&str>,
) -> ArgoResult<Result<RunOutcome, RunBlocked>> {
    // ── Validate-before-claim #1: a real provider must be configured.
    if !crate::handlers::gateway::startup_check::provider_key_present() {
        return Ok(Err(RunBlocked::NoProvider));
    }
    // ── Validate-before-claim #2: the worker's vertical must load,
    // tenant-scoped — a Genesis-hired teammate lives under
    // `.genesis/<tenant>/<vid>/`, NOT the flat catalog, and must resolve
    // ONLY for its own tenant (fail-closed cross-tenant).
    let vertical_id = worker.vertical_id.as_str();
    let Some(loaded) = load_worker_vertical(&worker.tenant_id.to_string(), vertical_id) else {
        return Ok(Err(RunBlocked::NoVertical));
    };

    // Filter the operator's pre-approval list to the worker's declared
    // tool set (no privilege escalation through `approve_tools`).
    let approved_tools = filter_to_worker_tool_set(&loaded.manifest, approve_tools);
    let pre_approved = (!approved_tools.is_empty()).then(|| {
        crate::handlers::work_dispatch::PreApprovedTools {
            tools: approved_tools.clone(),
            approved_by: approved_by.map(str::to_string),
        }
    });

    // Both preconditions pass — it is now honest to start a turn.
    let memory = MemoryStore::new(Arc::clone(store));
    let workspace = resolve_workspace(store, worker).await?;
    let (job, _created_at) =
        crate::handlers::work::append_pending_job(&memory, workspace, vertical_id, prompt).await?;

    // Run ONE real turn via the proven engine (NOT a parallel runner).
    // Stamp the run with `worker.id` so the session — and every turn
    // audit row emitted under it — attributes to THIS teammate instead
    // of resolving to "unknown" in the audit feed (e2e R2-F1).
    let (provider, model) = resolve_run_provider();
    crate::handlers::work_dispatch::process_job(
        store,
        &memory,
        worker.tenant_id,
        workspace,
        Some(worker.id),
        vertical_id,
        &provider,
        &model,
        &job,
        MAX_STEPS,
        MAX_TOOL_ROUNDS,
        /* json_out = */ true,
        pre_approved.as_ref(),
    )
    .await?;

    // Read back the completion row the engine wrote. It only exists if
    // the turn genuinely completed, so its presence IS the evidence.
    let mut outcome =
        read_completion(&memory, workspace, vertical_id, &job.id.to_string()).await?;
    // Stamp the claimed job id so the response can deep-link the run's
    // artifact bundle. Only meaningful on the completed path (a failed
    // run wrote no bundle); the SPA gates on `status == "completed"`.
    if outcome.status == "completed" {
        outcome.job_id = job.id.to_string();
    }
    outcome.approved_tools = approved_tools;
    Ok(Ok(outcome))
}

/// Resolve `vertical_id` to its loaded vertical for `tenant_id` — flat
/// in `verticals/<vid>/` OR tenant-scoped under
/// `verticals/.genesis/<tenant>/<vid>/`. Mirrors the validation `argo
/// work add` does, so a worker whose role was never materialized can't
/// claim a run; a Genesis pack belonging to a *different* tenant
/// resolves to `None` (fail-closed cross-tenant).
fn load_worker_vertical(
    tenant_id: &str,
    vertical_id: &str,
) -> Option<argo_vertical::LoadedVertical> {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    argo_vertical::load_for_worker(&dir, tenant_id, vertical_id)
}

/// Keep only the requested tools the worker would legitimately hold —
/// the membership filter under `approve_tools` (no privilege
/// escalation; the adversarial bar).
///
/// "Legitimately hold" mirrors the runtime's own least-privilege
/// derivation (`argo_agent_runner`'s `derive_profile_from_manifest` +
/// `WorkerCapabilityProfile::to_capability_set`): the manifest's
/// declared `[actions]` names + `[tools].allowed`, with `[tools].denied`
/// always winning, plus the always-safe
/// [`argo_capability::MINIMAL_TEAMMATE_TOOLS`] (unless denied). A
/// requested tool outside that set is SKIPPED — silently dropped, never
/// granted, and never an error that would fail the run.
///
/// Output is de-duplicated, preserving first-request order.
fn filter_to_worker_tool_set(
    manifest: &argo_vertical::VerticalManifest,
    requested: &[String],
) -> Vec<String> {
    let declared_actions: Vec<String> = manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().cloned().collect())
        .unwrap_or_default();
    let (allowed, denied): (Vec<String>, Vec<String>) = manifest
        .tools
        .as_ref()
        .map(|t| (t.allowed.clone(), t.denied.clone()))
        .unwrap_or_default();
    let profile = argo_capability::WorkerCapabilityProfile::derive(
        declared_actions,
        allowed,
        denied.clone(),
        Vec::<String>::new(),
    );
    let mut out: Vec<String> = Vec::new();
    for raw in requested {
        let tool = raw.trim();
        if tool.is_empty() || out.iter().any(|t| t == tool) {
            continue;
        }
        let minimal_safe = argo_capability::MINIMAL_TEAMMATE_TOOLS.contains(&tool)
            && !denied.iter().any(|d| d == tool);
        if profile.permits_tool(tool) || minimal_safe {
            out.push(tool.to_string());
        }
    }
    out
}

/// Resolve the workspace that owns this worker's tenant. Prefers the
/// tenant-scoped workspace (so a signed-in stranger's run anchors to
/// THEIR workspace, not the demo/cwd one). Falls back to the cwd
/// workspace only when the tenant has none registered yet (the demo /
/// anonymous path, where the worker's tenant IS the cwd tenant).
async fn resolve_workspace(store: &Arc<SessionStore>, worker: &Worker) -> ArgoResult<WorkspaceId> {
    if let Some(ws) = store.first_workspace_for_tenant(worker.tenant_id).await? {
        return Ok(ws);
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let (_tenant, ws) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .map_err(|e| ArgoError::Provider(format!("ensure_workspace: {e}")))?;
    Ok(ws)
}

/// Read back the `completed_jobs` memory row `process_job` wrote for
/// `pending_job_id`. Presence of the row is the truth signal: found →
/// `completed` with the persisted session + reply length; absent →
/// `failed` (the turn errored before recording a completion).
async fn read_completion(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_id: &str,
    pending_job_id: &str,
) -> ArgoResult<RunOutcome> {
    let entries = memory
        .list_for_vertical(workspace, MemoryFile::Memory, vertical_id)
        .await?;
    for e in entries.iter().rev() {
        let Ok(body) = serde_json::from_str::<Value>(&e.body) else {
            continue;
        };
        if body.get("schema_key").and_then(Value::as_str) != Some("completed_jobs") {
            continue;
        }
        if body.get("pending_job_id").and_then(Value::as_str) != Some(pending_job_id) {
            continue;
        }
        let session_id = body
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let chars = body
            .get("assistant_text")
            .and_then(Value::as_str)
            .map(str::chars)
            .map(Iterator::count)
            .unwrap_or(0);
        return Ok(RunOutcome {
            job_id: String::new(),
            session_id,
            status: "completed",
            chars,
            approved_tools: Vec::new(),
        });
    }
    // No completion row — the turn did not finish. Report it honestly.
    Ok(RunOutcome {
        job_id: String::new(),
        session_id: String::new(),
        status: "failed",
        chars: 0,
        approved_tools: Vec::new(),
    })
}

#[cfg(test)]
#[path = "routes_worker_run_exec_tests.rs"]
mod tests;
