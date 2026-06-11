//! `POST /api/v1/workers/<id>/run` — manual "Run now" trigger for the
//! demo runbook's Scene 2 ("click Mariah → Run now → audit row lands
//! in /audit within 5s"). Originally authored under
//! `fix-worker-detail-run-now-action-and-endpoint`; the real-run wire
//! lives in the sibling [`routes_worker_run_exec`].
//!
//! Pipeline:
//! 1. Parse + validate the path worker id.
//! 2. Resolve the workspace tenant + load the worker (404 / 403 / 410
//!    if unknown, cross-tenant, or archived).
//! 3. Capability check: re-use [`Action::WorkerLifecycle`] (the same
//!    admin-floor capability that gates pause/resume) per the brief's
//!    "`permissions.can_pause_resume` is a reasonable proxy" guidance.
//! 4. Parse body (`{ note?: string, trigger_source?: string,
//!    approve_tools?: string[] }`, all optional). `approve_tools` is
//!    the operator's one-action "Approve & run" pre-approval list: the
//!    exec layer filters it to the worker's declared tool set and mints
//!    ONE-SHOT session grants on the new run's session before the turn,
//!    so a probationary teammate's held tools clear in the same click
//!    that triggers the run (the approve→run loop converges).
//! 5. **Run a REAL turn** via [`routes_worker_run_exec::run_worker_now`]
//!    — validate-before-claim (worker has a loadable vertical + a
//!    provider key is present, else an honest `409 Conflict`), then run
//!    one turn through the proven
//!    [`crate::handlers::work_dispatch::process_job`] engine. No
//!    theater: the run is awaited and the response reports the TRUE
//!    outcome.
//! 6. Stamp a fresh `trigger_id` (UUIDv7) into an audit payload that
//!    carries the REAL outcome (`status`, `session_id`, `chars`) and
//!    emit a `worker.run` audit row via [`AuditSink`].
//! 7. Look up the audit row's primary key by `trigger_id` and return
//!    `{ action_id, audit_id, trigger_id, session_id, status, chars,
//!    queued_at }`. `action_id` and `audit_id` carry the same lower-hex
//!    32-char audit row id so the SPA toast can deep-link to
//!    `/audit/action/<action_id>` without guessing which field to read.
//!    The original `action_id` / `audit_id` / `trigger_id` fields are
//!    preserved for backward compatibility.
//!
//! ## Truth-bounded (NO THEATER)
//!
//! The earlier version of this handler wrote a `worker.run` marker row
//! and returned `queued` WITHOUT running anything — its own docs
//! admitted "the 'queued turn' isn't actually queued." That theater is
//! gone: a run is only claimed when a turn genuinely starts, and the
//! reported `status` is sourced from the engine's `completed_jobs`
//! record (`completed`) or its absence (`failed`) — never asserted
//! optimistically. See [`routes_worker_run_exec`] for the design note
//! (synchronous await, validate-before-claim, honest 409).
//!
//! ## Posture notes (carried from the original brief)
//!
//! - No new `Action::WorkerRun` variant; reusing `WorkerLifecycle`
//!   matches the brief's permissions-proxy guidance and avoids a
//!   workspace-wide enum match churn.
//! - The `"run"` path dispatch lives in `routes_dispatch_post_paths.rs`
//!   (the worker-lifecycle path-dispatcher) for pattern consistency.

use super::role_gate::{enforce_role, Action};
use super::routes_auth_principal::session_principal_id_for_audit;
use super::{body_from_request, store_unavailable};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

const RECOVERABLE_DAYS: i64 = 30;

/// `POST /api/v1/workers/<id>/run` — see module docs.
pub(super) async fn post_run(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::WorkerLifecycle) {
        return deny;
    }
    let Some(store) = store else {
        return store_unavailable();
    };
    let (tenant, worker) = match preflight(worker_id_raw, req, store).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let body = match parse_body(req) {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    let note = body.get("note").and_then(Value::as_str).map(str::to_string);
    let trigger_source = body
        .get("trigger_source")
        .and_then(Value::as_str)
        .unwrap_or("manual_run_now")
        .to_string();
    // "Approve & run" (keystone fix): optional operator pre-approval
    // list. Absent / non-array → empty (existing callers unaffected).
    // The exec layer filters it to the worker's declared tool set and
    // mints ONE-SHOT grants on the new run's session before the turn.
    // Cap the input array (defense-in-depth, adversarial-review note):
    // the downstream membership filter is already bounded by the worker's
    // (small) tool set, but bounding the raw input keeps the O(n) filter
    // loop cheap regardless of body size. 64 ≫ any real worker tool set.
    const MAX_APPROVE_TOOLS: usize = 64;
    let approve_tools: Vec<String> = body
        .get("approve_tools")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .take(MAX_APPROVE_TOOLS)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    // Run a REAL turn (validate-before-claim inside). A blocked
    // precondition returns an honest 409 — no turn, no marker.
    let prompt = run_prompt(&worker, note.as_deref());
    let approver = session_principal_id_for_audit(req);
    let outcome = match routes_worker_run_exec::run_worker_now(
        store,
        &worker,
        &prompt,
        &approve_tools,
        approver.as_deref(),
    )
    .await
    {
        Ok(Ok(o)) => o,
        Ok(Err(blocked)) => return conflict(blocked.code(), blocked.message()),
        Err(e) => return server_error(&format!("run worker: {e}")),
    };
    finish_run(store, req, &tenant, &worker, &trigger_source, note.as_deref(), &outcome).await
}

/// Emit the truthful `worker.run` audit row, resolve its id, and shape
/// the response. Split out of [`post_run`] to keep both fns < 80 LOC.
async fn finish_run(
    store: &Arc<SessionStore>,
    req: &str,
    tenant: &TenantId,
    worker: &Worker,
    trigger_source: &str,
    note: Option<&str>,
    outcome: &routes_worker_run_exec::RunOutcome,
) -> (&'static str, String) {
    let actor = session_principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let trigger_id = Uuid::now_v7().to_string();
    let queued_at = Utc::now();
    let payload = build_payload(
        worker,
        &trigger_id,
        trigger_source,
        note,
        &actor,
        queued_at,
        outcome,
    );
    if let Err(e) = write_audit(store, req, worker, payload).await {
        return server_error(&format!("audit write: {e}"));
    }
    let audit_hex = match lookup_audit_id(store, tenant, &trigger_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::warn!(
                target: "gateway::worker_run",
                trigger_id = %trigger_id,
                "audit row written but lookup returned no match"
            );
            return server_error("audit lookup: no row found after write");
        }
        Err(e) => return server_error(&format!("audit lookup: {e}")),
    };
    // Day-1 unblock fix 2 (additive): surface probation holds to the SPA.
    // `held_tools` is derived from the ProbationHold audit rows the
    // dispatch gate wrote for THIS run's session (the honest source —
    // never the model's say-so); `probation` reports whether the worker
    // was inside its probation window for this run.
    let held_tools = held_tools_for_session(store, tenant, &outcome.session_id).await;
    let probation =
        argo_identity::is_in_probation(&worker.persona.provenance_chain, Utc::now());
    let body = json!({
        "action_id": audit_hex,
        "audit_id": audit_hex,
        "trigger_id": trigger_id,
        "session_id": outcome.session_id,
        "status": outcome.status,
        "chars": outcome.chars,
        "queued_at": queued_at.to_rfc3339(),
        "held_tools": held_tools,
        "probation": probation,
        // "Approve & run": the requested tools that passed the
        // worker-tool-set membership filter and were minted as ONE-SHOT
        // grants on this run's session. Requested-but-skipped tools are
        // absent — the SPA can diff to see what was refused.
        "approved_tools": outcome.approved_tools,
    });
    ("200 OK", body.to_string())
}

/// Unique tool names the probation gate held during `session_id`'s run,
/// in first-hold order — read back from the `probation.hold` audit rows
/// [`argo_agent_runner`]'s dispatch gate wrote (`emit_probation_hold`).
/// Empty when nothing was held, when the run recorded no session, or on
/// any read error (additive response field — never fails the response).
async fn held_tools_for_session(
    store: &Arc<SessionStore>,
    tenant: &TenantId,
    session_id: &str,
) -> Vec<String> {
    let Ok(session_uuid) = Uuid::parse_str(session_id) else {
        return Vec::new();
    };
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE tenant_id = ? AND session_id = ? AND kind = 'probation.hold' \
         ORDER BY created_at ASC",
    )
    .bind(tenant.to_string())
    .bind(session_uuid.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .unwrap_or_default();
    let mut tools: Vec<String> = Vec::new();
    for raw in rows {
        let Some(tool) = serde_json::from_str::<Value>(&raw)
            .ok()
            .and_then(|v| v.get("tool").and_then(Value::as_str).map(str::to_string))
        else {
            continue;
        };
        if !tools.contains(&tool) {
            tools.push(tool);
        }
    }
    tools
}

/// The turn prompt: the operator's `note` when it carries text, else a
/// generic standing-task nudge derived from nothing worker-specific
/// (we never fabricate task specifics — the worker's own role + memory
/// supply the substance once the turn runs).
fn run_prompt(_worker: &Worker, note: Option<&str>) -> String {
    match note.map(str::trim) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => "Do your next standing task now and report what you did.".to_string(),
    }
}

async fn preflight(
    worker_id_raw: &str,
    req: &str,
    store: &Arc<SessionStore>,
) -> Result<(TenantId, Worker), (&'static str, String)> {
    let worker_id = parse_worker_id(worker_id_raw)?;
    // Resolve via the canonical per-request resolver (#2676 / sprint
    // WS1-2): a signed-in user's "Run now" must resolve THEIR JWT tenant
    // so the worker row they're acting on is found under the same tenant
    // their /workers and /team reads query. The old cwd resolver looked
    // up the gateway demo tenant, so a stranger's worker was invisible.
    // No session (demo/anonymous) still falls through to the cwd tenant.
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    let worker = load_worker(store, tenant, worker_id).await?;
    if let Some(archived) = fetch_archived_at(store, tenant, worker_id)
        .await
        .map_err(|e| server_error(&format!("archived lookup: {e}")))?
    {
        return Err(gone_response(worker_id, &archived));
    }
    Ok((tenant, worker))
}

fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let cleaned = raw.split('?').next().unwrap_or(raw);
    cleaned
        .parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{cleaned}': {e}")))
}

async fn load_worker(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> Result<Worker, (&'static str, String)> {
    match IdentityStore::new(store).get_worker(tenant, worker_id).await {
        Ok(Some(w)) => Ok(w),
        Ok(None) => Err(not_found("worker not found")),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

async fn fetch_archived_at(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<Option<DateTime<Utc>>> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT archived_at FROM worker \
         WHERE id = ? AND tenant_id = ? AND archived_at IS NOT NULL",
    )
    .bind(worker_id.to_string())
    .bind(tenant.to_string())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("archived_at lookup: {e}")))?;
    Ok(raw.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }))
}

fn parse_body(req: &str) -> Result<Value, (&'static str, String)> {
    let body = body_from_request(req);
    if body.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str::<Value>(body).map_err(|e| bad_request(&format!("invalid JSON body: {e}")))
}

#[allow(clippy::too_many_arguments)]
fn build_payload(
    worker: &Worker,
    trigger_id: &str,
    trigger_source: &str,
    note: Option<&str>,
    actor: &str,
    queued_at: DateTime<Utc>,
    outcome: &routes_worker_run_exec::RunOutcome,
) -> Value {
    // The audit row reflects the REAL outcome: a "run completed" /
    // "run failed" breadcrumb, not a generic "queued" marker.
    let mut decision = json!({
        "step": "manual_run",
        "actor": actor,
        "trigger_source": trigger_source,
        "status": outcome.status,
        "session_id": outcome.session_id,
        "chars": outcome.chars,
        "ts": queued_at.to_rfc3339(),
    });
    if let Some(n) = note {
        if let Some(obj) = decision.as_object_mut() {
            obj.insert("note".into(), json!(n));
        }
    }
    json!({
        // `verb` + `action_type` give the row a worker-run identity the
        // /audit feed renders and the default meta-filter does NOT hide.
        // Without them the row resolved its verb from the `http.request`
        // kind column, so it was both mislabeled and meta-filtered out of
        // the default feed (#2661). "worker.run" is not a metadata verb.
        "verb": "ran a manual turn",
        "action_type": "worker.run",
        "endpoint": "worker.run",
        "worker_id": worker.id.to_string(),
        "worker_name": worker.persona.name,
        "trigger_id": trigger_id,
        "trigger_source": trigger_source,
        "note": note.unwrap_or(""),
        // `outcome_status` is the canonical badge value both readers
        // normalize (`succeeded`/`failed`); the engine's raw `completed`
        // stays in `status` for the response/decision-chain breadcrumb.
        // Emitting only `completed` made a TRUE run render `Failed`
        // (the web `normalizeStatus` default).
        "outcome_status": canonical_outcome_status(outcome.status),
        "status": outcome.status,
        "session_id": outcome.session_id,
        "chars": outcome.chars,
        "queued_at": queued_at.to_rfc3339(),
        "decision_chain": [decision],
    })
}

/// Map the engine's `completed`/`failed` run outcome to the canonical
/// audit status vocabulary the readers normalize (`succeeded`/`failed`).
///
/// The engine reports `"completed"`, but both audit readers — the feed's
/// `routes_audit_feed::outcome_status` consumer and the web
/// `web/lib/api/audit-detail.ts::normalizeStatus` — only recognize
/// `"succeeded"`, defaulting anything else to `"failed"`. Stamping the
/// canonical value at the emit means no reader has to guess, so a TRUE
/// run never renders as `Failed`.
fn canonical_outcome_status(engine_status: &str) -> &'static str {
    match engine_status {
        "completed" => "succeeded",
        _ => "failed",
    }
}

async fn write_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    payload: Value,
) -> ArgoResult<()> {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), worker.tenant_id.to_string());
    sink.write(
        None,
        AuditKind::HttpRequest,
        payload,
        session_principal_id_for_audit(req),
    )
    .await
}

async fn lookup_audit_id(
    store: &Arc<SessionStore>,
    tenant: &TenantId,
    trigger_id: &str,
) -> ArgoResult<Option<String>> {
    let needle = format!("%\"trigger_id\":\"{trigger_id}\"%");
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM audit_record \
         WHERE tenant_id = ? AND payload_json LIKE ? \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(tenant.to_string())
    .bind(needle)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit lookup: {e}")))?;
    Ok(row.map(hex::encode))
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
}

/// Honest "can't run yet" response — the worker has no loadable role,
/// or no model provider is configured. A real precondition the
/// operator must fix, NOT a fabricated success (truth-bounded).
fn conflict(code: &str, message: &str) -> (&'static str, String) {
    error_envelope("409 Conflict", code, message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn gone_response(
    worker_id: WorkerId,
    archived_at: &DateTime<Utc>,
) -> (&'static str, String) {
    let recoverable = *archived_at + chrono::Duration::days(RECOVERABLE_DAYS);
    let body = json!({
        "error": { "code": "gone", "message": format!("worker {worker_id} is archived") },
        "archived_at": archived_at.to_rfc3339(),
        "recoverable_until_at": recoverable.to_rfc3339(),
    });
    ("410 Gone", body.to_string())
}

fn error_envelope(
    status: &'static str,
    code: &str,
    message: &str,
) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

/// Real-run execution path (validate-before-claim + `process_job`).
/// Mounted as a child module from the sibling file so the new
/// turn-running logic stays out of this file's LOC budget while the
/// HTTP plumbing remains here.
#[path = "routes_worker_run_exec.rs"]
mod routes_worker_run_exec;

#[cfg(test)]
#[path = "routes_worker_run_tests.rs"]
mod tests;
