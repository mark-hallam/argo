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
use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, MemoryScope, Worker, WorkerId};
use argo_memory::{MemoryFile, MemoryStore, PrincipalOverride};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

const INPUT_MAX_BYTES: usize = 4000;
const UNDO_TTL_DAYS: i64 = 14;
const REVERSE_TEST_MAX: u32 = 10;
const REVERSE_TEST_DEFAULT: u32 = 3;
const COMMIT_KEY: &str = "worker.retrain.committed";
const UNDO_KEY: &str = "worker.retrain.undone";
const TRUST_RESET_KEY: &str = "worker.retrain.trust_ratchet.reset";
const REVERSE_TEST_KEY: &str = "worker.retrain.reverse_tested";
const FACT_TOMBSTONE: &str = "\u{0007}skycot-fact-tombstone\u{0007}";
const KNOWN_CLASSES: [&str; 5] = [
    "memory_fact", "policy_rule", "skill_correction", "vocabulary_patch", "action_ban",
];

type Resp = (&'static str, String);
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

struct ClassifyOutcome {
    classification: String,
    plain_english: String,
    structural_change: Value,
    needs_generalization: Option<Value>,
}
fn classify_input(input: &str) -> ClassifyOutcome {
    let lower = input.to_ascii_lowercase();
    if let Some((from, to)) = detect_vocabulary_patch(&lower) {
        return outcome("vocabulary_patch", format!("Replace '{from}' with '{to}'."),
            json!({ "from": from, "to": to }), None);
    }
    if let Some(banned) = detect_action_ban(&lower) {
        return outcome("action_ban", format!("Ban action '{banned}'."),
            json!({ "action_type": banned }), None);
    }
    if matches_any(&lower, &["always ", "must ", "must not ", "before ", "require ", "only when "]) {
        return outcome("policy_rule",
            format!("Add policy rule: {}", trim_for_display(input)),
            json!({ "predicate_json": { "raw": input }, "action_type": guess_action_type(&lower) }),
            None);
    }
    if matches_any(&lower, &["you got it wrong", "next time", "incorrect", "wrong answer"]) {
        return outcome("skill_correction",
            format!("Correct skill output: {}", trim_for_display(input)),
            json!({ "note": input }),
            Some(json!({
                "question": "Apply to all similar inputs or only this exact case?",
                "options": ["all similar", "this case only"],
            })));
    }
    outcome("memory_fact",
        format!("Remember: {}", trim_for_display(input)),
        json!({ "text": input }),
        None)
}
fn outcome(c: &str, p: String, s: Value, g: Option<Value>) -> ClassifyOutcome {
    ClassifyOutcome { classification: c.into(), plain_english: p, structural_change: s, needs_generalization: g }
}
fn matches_any(lower: &str, keys: &[&str]) -> bool { keys.iter().any(|k| lower.contains(k)) }
fn detect_vocabulary_patch(lower: &str) -> Option<(String, String)> {
    let idx = lower.find(" instead of ")?;
    let head = &lower[..idx];
    let new_word = head.rsplit_once(' ').map_or(head, |(_, w)| w);
    let tail = &lower[idx + " instead of ".len()..];
    let old_word = tail.split([' ', '.', ',']).next().unwrap_or(tail);
    (!new_word.is_empty() && !old_word.is_empty())
        .then(|| (old_word.to_string(), new_word.to_string()))
}

fn detect_action_ban(lower: &str) -> Option<String> {
    for prefix in ["stop doing ", "don't ever ", "never "] {
        if let Some(idx) = lower.find(prefix) {
            let tail = &lower[idx + prefix.len()..];
            let first = tail.split([' ', '.', ',']).next().unwrap_or(tail);
            if !first.is_empty() {
                return Some(first.to_string());
            }
        }
    }
    None
}

fn guess_action_type(lower: &str) -> String {
    for kw in ["send_email", "send_sms", "send", "post", "pay", "refund", "schedule"] {
        if lower.contains(kw) { return kw.into(); }
    }
    "unspecified".into()
}

fn trim_for_display(input: &str) -> String {
    let s = input.trim();
    if s.len() <= 120 { s.into() } else { format!("{}…", s.chars().take(117).collect::<String>()) }
}
async fn find_conflicts(
    store: &Arc<SessionStore>, ctx: &WorkerCtx, outcome: &ClassifyOutcome, input: &str,
) -> ArgoResult<Vec<Value>> {
    if !matches!(outcome.classification.as_str(), "memory_fact" | "vocabulary_patch") {
        return Ok(vec![]);
    }
    let lower = input.to_ascii_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4).take(8).collect();
    if words.is_empty() { return Ok(vec![]); }
    let ns = &ctx.worker.memory_namespace;
    let rows: Vec<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT id, body FROM memory_entry \
         WHERE tenant_id = ? AND worker_id = ? AND memory_scope = ? \
           AND file = ? AND superseded_by IS NULL AND body != ? \
         ORDER BY learned_at DESC LIMIT 100",
    )
    .bind(ns.tenant_id.0.as_bytes().as_slice())
    .bind(ns.worker_id.0.as_bytes().as_slice())
    .bind(scope_label(ns.scope))
    .bind(MemoryFile::Memory.filename())
    .bind(FACT_TOMBSTONE)
    .fetch_all(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("conflict scan: {e}")))?;
    let mut conflicts = Vec::new();
    for (id_bytes, body) in rows {
        let lb = body.to_ascii_lowercase();
        let overlaps = words.iter().filter(|w| lb.contains(*w)).count();
        if overlaps >= 2 {
            let id = Uuid::from_slice(&id_bytes)
                .map_err(|e| ArgoError::Provider(format!("fact id: {e}")))?;
            conflicts.push(json!({ "fact_id": id.to_string(), "existing_text": body, "overlap_words": overlaps }));
            if conflicts.len() >= 5 { break; }
        }
    }
    Ok(conflicts)
}

fn capture_snapshot(ctx: &WorkerCtx, classification: &str, sc: &Value) -> Value {
    match classification {
        "memory_fact" => json!({ "kind": "memory_fact" }),
        "policy_rule" => json!({ "kind": "policy_rule", "rule_intent": sc }),
        "voice_profile_edit" => json!({
            "kind": "voice_profile_edit",
            "persona": serde_json::to_value(&ctx.worker.persona).unwrap_or(Value::Null),
        }),
        "vocabulary_patch" => json!({ "kind": "vocabulary_patch" }),
        "action_ban" => json!({
            "kind": "action_ban",
            "action_type": sc.get("action_type").cloned().unwrap_or(Value::Null),
        }),
        _ => Value::Null,
    }
}
async fn apply_memory_fact(store: &Arc<SessionStore>, ctx: &WorkerCtx, sc: &Value) -> Result<(), Resp> {
    let text = sc.get("text").and_then(Value::as_str)
        .ok_or_else(|| bad_request("memory_fact requires `structural_change.text`"))?
        .trim().to_string();
    if text.is_empty() { return Err(bad_request("memory_fact text is empty")); }
    let workspace = crate::home::cwd_workspace(store).await
        .map_err(|e| server_error(&format!("cwd_workspace: {e}")))?;
    MemoryStore::new(Arc::clone(store))
        .append_in_namespace(&ctx.worker.memory_namespace, workspace, MemoryFile::Memory, text, None)
        .await.map(|_| ())
        .map_err(|e| server_error(&format!("memory append: {e}")))
}
async fn revert_memory_fact(store: &Arc<SessionStore>, row: &RetrainRow) -> Result<(), Resp> {
    let Some(text) = row.structural_change.get("text").and_then(Value::as_str) else { return Ok(()) };
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM memory_entry WHERE body = ? AND superseded_by IS NULL ORDER BY learned_at DESC LIMIT 1",
    )
    .bind(text).fetch_optional(store.pool()).await
    .map_err(|e| server_error(&format!("fact lookup: {e}")))?;
    let Some(b) = bytes else { return Ok(()) };
    let id = Uuid::from_slice(&b).map_err(|e| server_error(&format!("fact id: {e}")))?;
    MemoryStore::new(Arc::clone(store))
        .supersede(id, FACT_TOMBSTONE.into(), None, PrincipalOverride::Agent)
        .await.map(|_| ())
        .map_err(|e| server_error(&format!("fact supersede: {e}")))
}

struct RetrainRow {
    classification: String,
    structural_change: Value,
    applied_at: DateTime<Utc>,
    is_undone: bool,
}

async fn find_retrain_row(
    store: &Arc<SessionStore>, ctx: &WorkerCtx, retrain_id: Uuid,
) -> ArgoResult<Option<RetrainRow>> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT json_extract(payload_json, '$.classification'), \
                json_extract(payload_json, '$.structural_change'), \
                json_extract(payload_json, '$.applied_at') \
         FROM audit_record WHERE kind = 'http.request' \
           AND json_extract(payload_json, '$.endpoint') = ? \
           AND json_extract(payload_json, '$.retrain_id') = ? \
           AND json_extract(payload_json, '$.worker_id') = ? \
         LIMIT 1",
    )
    .bind(COMMIT_KEY).bind(retrain_id.to_string()).bind(ctx.worker.id.to_string())
    .fetch_optional(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("retrain row probe: {e}")))?;
    let Some((classification, sc_raw, applied_raw)) = row else { return Ok(None) };
    let structural_change: Value = serde_json::from_str(&sc_raw).unwrap_or(Value::Null);
    let applied_at = DateTime::parse_from_rfc3339(&applied_raw)
        .map_err(|e| ArgoError::Provider(format!("applied_at parse: {e}")))?
        .with_timezone(&Utc);
    let undone: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'http.request' \
           AND json_extract(payload_json, '$.endpoint') = ? \
           AND json_extract(payload_json, '$.retrain_id') = ?",
    )
    .bind(UNDO_KEY).bind(retrain_id.to_string())
    .fetch_one(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("undo probe: {e}")))?;
    Ok(Some(RetrainRow { classification, structural_change, applied_at, is_undone: undone > 0 }))
}

async fn build_reverse_samples(
    store: &Arc<SessionStore>, ctx: &WorkerCtx, row: &RetrainRow, sample_size: u32,
) -> Vec<Value> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT COALESCE(json_extract(payload_json, '$.tool_name'), ''), payload_json \
         FROM audit_record WHERE kind = 'tool_call' \
           AND json_extract(payload_json, '$.worker_id') = ? \
         ORDER BY seq DESC LIMIT ?",
    )
    .bind(ctx.worker.id.to_string()).bind(i64::from(sample_size).max(1))
    .fetch_all(store.pool()).await.unwrap_or_default();
    rows.into_iter().take(sample_size as usize).map(|(tn, payload)| {
        let tool_name = if tn.is_empty() { "unknown".into() } else { tn };
        let outcome = match row.classification.as_str() {
            "action_ban" if tool_name.contains("send") || tool_name.contains("post") => "blocked",
            "policy_rule" => "review",
            _ => "safe",
        };
        json!({
            "inbound_context": { "tool_name": tool_name, "raw_payload": payload },
            "before_output": { "action": "unchanged", "tool_name": tool_name },
            "after_output": { "action": outcome, "tool_name": tool_name },
            "outcome": outcome,
        })
    }).collect()
}
struct WorkerCtx {
    tenant: TenantId,
    worker: Worker,
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
fn scope_label(scope: MemoryScope) -> &'static str {
    match scope {
        MemoryScope::WorkerSelf => "worker_self",
        MemoryScope::Role => "role",
        MemoryScope::SharedTeam => "shared_team",
    }
}
async fn emit_audit(store: &Arc<SessionStore>, req: &str, payload: Value) {
    if let Err(e) = AuditSink::new(Arc::clone(store))
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
        "worker_id": ctx.worker.id.to_string(),
        "action_type": action_type,
        "phase": phase,
    })).await;
}
fn bad_request(m: &str) -> Resp { envelope("400 Bad Request", "bad_request", m) }
fn not_found(m: &str) -> Resp { envelope("404 Not Found", "not_found", m) }
fn server_error(m: &str) -> Resp { envelope("500 Internal Server Error", "internal_error", m) }
fn envelope(status: &'static str, code: &str, message: &str) -> Resp {
    (status, json!({ "error": { "code": code, "message": message } }).to_string())
}

#[cfg(test)]
#[path = "routes_retrain_tests.rs"]
mod tests;
