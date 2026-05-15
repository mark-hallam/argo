//! `POST /api/v1/workers/<id>/retrain/{classify,commit,undo,reverse-test}`
//! — Stream J12. Four JSON endpoints backing E4-g's RetrainDialog.
//!
//! v1 posture (full deviations documented in the PR body):
//! - Classifier is a local keyword heuristic; Genesis-lite targets a
//!   different taxonomy (charters → archetypes).
//! - Only `memory_fact` mutates a store; other classifications record
//!   an audit row + before-snapshot only (persona / policy / lexicon
//!   / action-allowed mutation primitives don't ship yet).
//! - Cross-tenant + unknown-worker both surface as 404 (mirrors J10).
//! - `regression_preview` is a stable empty array until per-action
//!   sample replay ships.
//! - Reverse-test is a deterministic structural diff (0 LLM calls).
//!
//! Classifier, conflict scan, snapshot capture, memory-fact apply/revert,
//! and the audit-table queries that hydrate `RetrainRow` live in the
//! `routes_retrain_helpers.rs` sibling (mounted as `helpers`) to keep
//! this file under the 500-LOC ceiling.
use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

#[path = "routes_retrain_helpers.rs"]
mod helpers;
use helpers::{
    apply_memory_fact, build_reverse_samples, capture_snapshot, classify_input, find_conflicts,
    find_retrain_row, revert_memory_fact, RetrainRow,
};

const INPUT_MAX_BYTES: usize = 4000;
const UNDO_TTL_DAYS: i64 = 14;
const REVERSE_TEST_MAX: u32 = 10;
const REVERSE_TEST_DEFAULT: u32 = 3;
pub(super) const COMMIT_KEY: &str = "worker.retrain.committed";
pub(super) const UNDO_KEY: &str = "worker.retrain.undone";
const TRUST_RESET_KEY: &str = "worker.retrain.trust_ratchet.reset";
const REVERSE_TEST_KEY: &str = "worker.retrain.reverse_tested";
const KNOWN_CLASSES: [&str; 5] = [
    "memory_fact", "policy_rule", "skill_correction", "vocabulary_patch", "action_ban",
];

pub(super) type Resp = (&'static str, String);
/// Inner-handler result. `Err` short-circuits with an HTTP envelope;
/// the public wrappers flatten via `unwrap_or_else(|r| r)`.
type RespResult = Result<Resp, Resp>;

/// Routes `POST /api/v1/workers/<id>/retrain/<sub>`. Single delegate
/// keeps `routes_dispatch_post.rs` near its 380-LOC post-split ceiling.
pub(super) async fn dispatch(req: &str, store: Option<&Arc<SessionStore>>) -> Option<Resp> {
    let rest = req.strip_prefix("POST /api/v1/workers/")?;
    let (head, _) = rest.split_once(' ')?;
    let parts: Vec<&str> = head.split('/').collect();
    if parts.len() != 3 || parts[1] != "retrain" {
        return None;
    }
    let id = parts[0];
    Some(match parts[2] {
        "classify" => classify_response(id, req, store).await,
        "commit" => commit_response(id, req, store).await,
        "undo" => undo_response(id, req, store).await,
        "reverse-test" => reverse_test_response(id, req, store).await,
        _ => return None,
    })
}
pub(super) async fn classify_response(w: &str, r: &str, s: Option<&Arc<SessionStore>>) -> Resp { classify_inner(w, r, s).await.unwrap_or_else(|x| x) }
pub(super) async fn commit_response(w: &str, r: &str, s: Option<&Arc<SessionStore>>) -> Resp { commit_inner(w, r, s).await.unwrap_or_else(|x| x) }
pub(super) async fn undo_response(w: &str, r: &str, s: Option<&Arc<SessionStore>>) -> Resp { undo_inner(w, r, s).await.unwrap_or_else(|x| x) }
pub(super) async fn reverse_test_response(w: &str, r: &str, s: Option<&Arc<SessionStore>>) -> Resp { reverse_test_inner(w, r, s).await.unwrap_or_else(|x| x) }
async fn classify_inner(wid: &str, req: &str, store: Option<&Arc<SessionStore>>) -> RespResult {
    let (store, ctx, input) = prepare_with_input(store, wid, req).await?;
    let outcome = classify_input(&input);
    let conflicts = find_conflicts(store, &ctx, &outcome, &input).await
        .map_err(|e| server_error(&format!("conflict scan: {e}")))?;
    Ok(("200 OK", json!({
        "classification": outcome.classification,
        "plain_english": outcome.plain_english,
        "structural_change": outcome.structural_change,
        "conflicts": conflicts,
        "needs_generalization": outcome.needs_generalization,
        "regression_preview": Vec::<Value>::new(),
    }).to_string()))
}

async fn commit_inner(wid: &str, req: &str, store: Option<&Arc<SessionStore>>) -> RespResult {
    let (store, ctx, parsed) = prepare(store, wid, req).await?;
    let classification = parsed.get("classification").and_then(Value::as_str)
        .filter(|s| KNOWN_CLASSES.contains(s))
        .ok_or_else(|| bad_request("missing or unknown `classification`"))?
        .to_string();
    let structural_change = parsed.get("structural_change").filter(|v| v.is_object())
        .cloned()
        .ok_or_else(|| bad_request("`structural_change` (object) is required"))?;
    let conflict_resolution = parsed.get("accepted_conflict_resolution").cloned();
    let before_snapshot = capture_snapshot(&ctx, &classification, &structural_change);
    if classification == "memory_fact" {
        apply_memory_fact(store, &ctx, &structural_change).await?;
    }
    let retrain_id = Uuid::now_v7();
    let applied_at = Utc::now();
    emit_audit(store, req, json!({
        "endpoint": COMMIT_KEY,
        "retrain_id": retrain_id.to_string(),
        "tenant_id": ctx.tenant.to_string(),
        "worker_id": ctx.worker.id.to_string(),
        "classification": classification,
        "structural_change": structural_change,
        "accepted_conflict_resolution": conflict_resolution,
        "before_snapshot": before_snapshot,
        "applied_at": applied_at.to_rfc3339(),
    })).await;
    if classification == "policy_rule" {
        emit_trust_reset(store, req, retrain_id, &ctx, &structural_change, "commit").await;
    }
    Ok(("201 Created", json!({
        "retrain_id": retrain_id.to_string(),
        "applied_at": applied_at.to_rfc3339(),
    }).to_string()))
}

async fn undo_inner(wid: &str, req: &str, store: Option<&Arc<SessionStore>>) -> RespResult {
    let (store, ctx, parsed) = prepare(store, wid, req).await?;
    let retrain_id = parse_retrain_id(&parsed)?;
    let row = require_row(store, &ctx, retrain_id).await?;
    if row.is_undone {
        return Err(bad_request("retrain has already been undone"));
    }
    if Utc::now() - row.applied_at > chrono::Duration::days(UNDO_TTL_DAYS) {
        return Err(envelope("410 Gone", "undo_window_expired",
            &format!("undo window ({UNDO_TTL_DAYS}d) elapsed")));
    }
    if row.classification == "memory_fact" {
        revert_memory_fact(store, &row).await?;
    }
    let undone_at = Utc::now();
    emit_audit(store, req, json!({
        "endpoint": UNDO_KEY,
        "retrain_id": retrain_id.to_string(),
        "tenant_id": ctx.tenant.to_string(),
        "worker_id": ctx.worker.id.to_string(),
        "classification": row.classification,
        "undone_at": undone_at.to_rfc3339(),
    })).await;
    if row.classification == "policy_rule" {
        emit_trust_reset(store, req, retrain_id, &ctx, &row.structural_change, "undo").await;
    }
    Ok(("200 OK", json!({ "ok": true, "undone_at": undone_at.to_rfc3339() }).to_string()))
}

async fn reverse_test_inner(wid: &str, req: &str, store: Option<&Arc<SessionStore>>) -> RespResult {
    let (store, ctx, parsed) = prepare(store, wid, req).await?;
    let retrain_id = parse_retrain_id(&parsed)?;
    let sample_size = match parsed.get("sample_size") {
        Some(v) => match v.as_u64() {
            Some(n) if (1..=u64::from(REVERSE_TEST_MAX)).contains(&n) => n as u32,
            _ => return Err(bad_request(&format!("`sample_size` must be 1..={REVERSE_TEST_MAX}"))),
        },
        None => REVERSE_TEST_DEFAULT,
    };
    let row = require_row(store, &ctx, retrain_id).await?;
    let samples = build_reverse_samples(store, &ctx, &row, sample_size).await;
    emit_audit(store, req, json!({
        "endpoint": REVERSE_TEST_KEY,
        "retrain_id": retrain_id.to_string(),
        "tenant_id": ctx.tenant.to_string(),
        "worker_id": ctx.worker.id.to_string(),
        "sample_size": sample_size,
        "llm_calls_used": 0,
    })).await;
    Ok(("200 OK", json!({ "samples": samples }).to_string()))
}
async fn require_row(store: &Arc<SessionStore>, ctx: &WorkerCtx, retrain_id: Uuid) -> Result<RetrainRow, Resp> {
    match find_retrain_row(store, ctx, retrain_id).await {
        Ok(Some(r)) => Ok(r),
        Ok(None) => Err(not_found(&format!("retrain {retrain_id} not found"))),
        Err(e) => Err(server_error(&format!("retrain lookup: {e}"))),
    }
}
pub(super) struct WorkerCtx {
    pub(super) tenant: TenantId,
    pub(super) worker: Worker,
}
async fn prepare<'s>(
    store: Option<&'s Arc<SessionStore>>, wid: &str, req: &str,
) -> Result<(&'s Arc<SessionStore>, WorkerCtx, Value), Resp> {
    let store = store.ok_or_else(super::store_unavailable)?;
    let worker_id = parse_worker_id(wid)?;
    let parsed = parse_body(req)?;
    let ctx = resolve_worker_ctx(store, worker_id).await?;
    Ok((store, ctx, parsed))
}
async fn prepare_with_input<'s>(
    store: Option<&'s Arc<SessionStore>>, wid: &str, req: &str,
) -> Result<(&'s Arc<SessionStore>, WorkerCtx, String), Resp> {
    let (store, ctx, parsed) = prepare(store, wid, req).await?;
    let input = parsed.get("input").and_then(Value::as_str)
        .map(str::trim).filter(|s| !s.is_empty())
        .ok_or_else(|| bad_request("missing or empty `input`"))?
        .to_string();
    if input.len() > INPUT_MAX_BYTES {
        return Err(bad_request(&format!("`input` exceeds {INPUT_MAX_BYTES}-byte limit")));
    }
    Ok((store, ctx, input))
}
fn parse_worker_id(raw: &str) -> Result<WorkerId, Resp> {
    let t = raw.split('?').next().unwrap_or(raw);
    t.parse::<WorkerId>().map_err(|e| bad_request(&format!("invalid worker_id '{t}': {e}")))
}
fn parse_retrain_id(parsed: &Value) -> Result<Uuid, Resp> {
    let s = parsed.get("retrain_id").and_then(Value::as_str)
        .ok_or_else(|| bad_request("`retrain_id` is required"))?;
    Uuid::parse_str(s).map_err(|e| bad_request(&format!("invalid retrain_id: {e}")))
}
fn parse_body(req: &str) -> Result<Value, Resp> {
    let body = body_from_request(req);
    if body.is_empty() { return Ok(json!({})); }
    serde_json::from_str(body).map_err(|e| bad_request(&format!("parse JSON body: {e}")))
}

async fn resolve_worker_ctx(store: &Arc<SessionStore>, worker_id: WorkerId) -> Result<WorkerCtx, Resp> {
    let tenant = resolve_workspace_tenant(store).await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    match IdentityStore::new(store).get_worker(tenant, worker_id).await {
        Ok(Some(worker)) => Ok(WorkerCtx { tenant, worker }),
        Ok(None) => Err(not_found(&format!("worker {worker_id} not found"))),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}
async fn emit_audit(store: &Arc<SessionStore>, req: &str, payload: Value) {
    let tenant_id = payload
        .get("tenant_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| SYSTEM_AUDIT_TENANT_ID.to_string());
    if let Err(e) = AuditSink::new_with_tenant(Arc::clone(store), tenant_id)
        .write(None, AuditKind::HttpRequest, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(target: "gateway::retrain", error = %e, "retrain audit write failed");
    }
}
async fn emit_trust_reset(
    store: &Arc<SessionStore>, req: &str, retrain_id: Uuid, ctx: &WorkerCtx, sc: &Value, phase: &str,
) {
    let action_type = sc.get("action_type").and_then(Value::as_str)
        .unwrap_or("unspecified").to_string();
    emit_audit(store, req, json!({
        "endpoint": TRUST_RESET_KEY,
        "retrain_id": retrain_id.to_string(),
        "tenant_id": ctx.tenant.to_string(),
        "worker_id": ctx.worker.id.to_string(),
        "action_type": action_type,
        "phase": phase,
    })).await;
}
pub(super) fn bad_request(m: &str) -> Resp { envelope("400 Bad Request", "bad_request", m) }
fn not_found(m: &str) -> Resp { envelope("404 Not Found", "not_found", m) }
pub(super) fn server_error(m: &str) -> Resp { envelope("500 Internal Server Error", "internal_error", m) }
fn envelope(status: &'static str, code: &str, message: &str) -> Resp {
    (status, json!({ "error": { "code": code, "message": message } }).to_string())
}

#[cfg(test)]
#[path = "routes_retrain_tests.rs"]
mod tests;
