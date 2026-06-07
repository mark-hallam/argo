//! Internal helpers for [`super`] (`routes_worker_memory`).
//!
//! Split out from the handler file to keep both under the 500-LOC
//! ceiling. Items are `pub(super)`-only and not part of the public
//! `gateway::routes_worker_memory::*` surface.

use super::super::routes_auth_principal::principal_id_for_audit;
use super::{FACT_MAX_BYTES, FACT_TOMBSTONE_MARKER, ROLE_HEADER};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, MemoryScope, Worker, WorkerId};
use argo_memory::{MemoryEntry, MemoryFile, MemoryStore};
use argo_redact::{redact_text, RedactionPolicy};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

// ----- lookup + validation -------------------------------------------------

pub(super) struct WorkerCtx {
    pub(super) tenant: TenantId,
    pub(super) worker: Worker,
}

pub(super) fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    t.parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{t}': {e}")))
}

pub(super) fn parse_fact_id(raw: &str) -> Result<Uuid, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    Uuid::parse_str(t).map_err(|e| bad_request(&format!("invalid fact_id '{t}': {e}")))
}

pub(super) fn parse_text_body(req: &str) -> Result<String, (&'static str, String)> {
    let parsed: Value = serde_json::from_str(super::super::body_from_request(req))
        .map_err(|e| bad_request(&format!("parse JSON body: {e}")))?;
    let raw = parsed
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("missing `text` field"))?
        .to_string();
    if raw.trim().is_empty() {
        return Err(bad_request("empty text"));
    }
    if raw.len() > FACT_MAX_BYTES {
        return Err(bad_request(&format!(
            "text exceeds {FACT_MAX_BYTES}-byte limit"
        )));
    }
    if raw.contains(FACT_TOMBSTONE_MARKER) {
        return Err(bad_request("text contains reserved tombstone marker"));
    }
    Ok(raw)
}

pub(super) async fn resolve_worker_ctx(
    store: &Arc<SessionStore>,
    req: &str,
    worker_id: WorkerId,
) -> Result<WorkerCtx, (&'static str, String)> {
    let tenant = super::super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    match IdentityStore::new(store)
        .get_worker(tenant, worker_id)
        .await
    {
        Ok(Some(worker)) => Ok(WorkerCtx { tenant, worker }),
        // Cross-tenant / cross-worker / unknown all surface as 404 —
        // per acceptance criteria, don't leak existence.
        Ok(None) => Err(not_found(&format!("worker {worker_id} not found"))),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

pub(super) fn parse_role_header(req: &str) -> Option<String> {
    let headers = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    headers.lines().skip(1).find_map(|line| {
        let (n, v) = line.split_once(':')?;
        n.eq_ignore_ascii_case(ROLE_HEADER)
            .then(|| v.trim().to_ascii_lowercase())
    })
}

pub(super) fn role_shows_ns(role: &str) -> bool {
    matches!(
        role,
        "operator" | "auditor" | "compliance-officer" | "compliance_officer"
    )
}

pub(super) fn format_namespace_id(worker: &Worker) -> String {
    let ns = &worker.memory_namespace;
    format!("{}/{}", ns.tenant_id, ns.worker_id)
}

fn scope_str(scope: MemoryScope) -> &'static str {
    match scope {
        MemoryScope::WorkerSelf => "worker_self",
        MemoryScope::Role => "role",
        MemoryScope::SharedTeam => "shared_team",
    }
}

// ----- fact storage --------------------------------------------------------

pub(super) struct FactRow {
    pub(super) id: Uuid,
    pub(super) body: String,
    pub(super) learned_at: DateTime<Utc>,
}

pub(super) async fn fetch_active_facts(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
) -> ArgoResult<Vec<FactRow>> {
    let ns = &ctx.worker.memory_namespace;
    let rows: Vec<(Vec<u8>, String, i64)> = sqlx::query_as(
        "SELECT id, body, learned_at FROM memory_entry \
         WHERE tenant_id = ? AND worker_id = ? AND memory_scope = ? \
           AND file = ? AND superseded_by IS NULL AND body != ? \
         ORDER BY learned_at ASC",
    )
    .bind(ns.tenant_id.0.as_bytes().as_slice())
    .bind(ns.worker_id.0.as_bytes().as_slice())
    .bind(scope_str(ns.scope))
    .bind(MemoryFile::Memory.filename())
    .bind(FACT_TOMBSTONE_MARKER)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("fact list: {e}")))?;
    rows.into_iter()
        .map(|(id_bytes, body, learned_at)| {
            let id = Uuid::from_slice(&id_bytes)
                .map_err(|e| ArgoError::Provider(format!("memory_entry.id parse: {e}")))?;
            let learned_at = Utc.timestamp_micros(learned_at).single().ok_or_else(|| {
                ArgoError::Provider(format!("memory_entry.learned_at oob: {learned_at}"))
            })?;
            Ok(FactRow {
                id,
                body,
                learned_at,
            })
        })
        .collect()
}

/// `Ok(None)` for unknown / cross-tenant / cross-worker / tombstoned
/// ids so the caller surfaces one uniform 404.
pub(super) async fn fetch_owned_active_fact(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    fact_id: Uuid,
) -> ArgoResult<Option<MemoryEntry>> {
    let memory = MemoryStore::new(Arc::clone(store));
    let entry = match memory.get(fact_id).await {
        Ok(e) => e,
        Err(ArgoError::Schema(_)) => return Ok(None),
        Err(e) => return Err(e),
    };
    if entry.file != MemoryFile::Memory
        || entry.superseded_by.is_some()
        || entry.body == FACT_TOMBSTONE_MARKER
    {
        return Ok(None);
    }
    let owner: Option<(Vec<u8>, Vec<u8>, String)> =
        sqlx::query_as("SELECT tenant_id, worker_id, memory_scope FROM memory_entry WHERE id = ?")
            .bind(fact_id.as_bytes().as_slice())
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("fact owner probe: {e}")))?;
    let Some((tenant_bytes, worker_bytes, scope)) = owner else {
        return Ok(None);
    };
    let ns = &ctx.worker.memory_namespace;
    let matches = tenant_bytes == ns.tenant_id.0.as_bytes()
        && worker_bytes == ns.worker_id.0.as_bytes()
        && scope == scope_str(ns.scope);
    Ok(matches.then_some(entry))
}

pub(super) fn fact_json(row: &FactRow, policy: &RedactionPolicy) -> Value {
    let (text, _) = redact_text(&row.body, policy);
    // camelCase, matching the sibling `active_conversations` /
    // `recent_decisions` arrays in the SAME payload and the SPA's
    // `WorkerFact` contract (`factId` / `createdAt`). The prior
    // snake_case `created_at` left the SPA's `new Date(fact.createdAt)`
    // reading `undefined` → "Invalid Date" on the Memory tab. The
    // snake_case keys are kept as aliases so any other reader that pinned
    // them keeps working.
    json!({
        "factId": row.id.to_string(),
        "fact_id": row.id.to_string(),
        "text": text,
        "createdAt": row.learned_at.to_rfc3339(),
        "created_at": row.learned_at.to_rfc3339(),
    })
}

// ----- active conversations ------------------------------------------------

/// Max conversation threads / recent decisions returned per worker. The
/// SPA cards cap their own render (`RecentDecisionsCard` at 10); this
/// bounds the read so the payload stays small.
const CONVERSATION_LIMIT: i64 = 10;
const DECISION_LIMIT: i64 = 10;

/// Read a worker's open conversation threads from the
/// `worker_conversation_thread` store (migration v87) and shape them into
/// the camelCase objects the `ActiveConversationsCard` renders directly
/// (`conversationId` / `counterpartyHandle` / `topic` / `lastMessageAt` /
/// `href`). The handle + topic are pre-redacted here (the card re-wraps
/// them in `<RedactedText>` for a second layer). Tenant + worker-scoped by
/// the store query — no cross-tenant / cross-worker leak.
pub(super) async fn fetch_active_conversations(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    policy: &RedactionPolicy,
) -> ArgoResult<Vec<Value>> {
    let ns = &ctx.worker.memory_namespace;
    let rows = argo_store::conversation_thread::list_for_worker(
        store.pool(),
        &ns.tenant_id.to_string(),
        ns.worker_id.0.as_bytes().as_slice(),
        CONVERSATION_LIMIT,
    )
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let (handle, _) = redact_text(&r.counterparty_handle, policy);
            let (topic, _) = redact_text(&r.topic, policy);
            let last = Utc
                .timestamp_micros(r.last_message_at)
                .single()
                .map_or_else(String::new, |dt| dt.to_rfc3339());
            json!({
                "conversationId": r.id,
                "counterpartyHandle": handle,
                "topic": topic,
                "lastMessageAt": last,
                "href": r.audit_href.unwrap_or_default(),
            })
        })
        .collect())
}

// ----- recent decisions ----------------------------------------------------

/// Derive a worker's recent material decisions from the audit log — the
/// approval log IS the decision record. Reads the worker's
/// `approval.asked` audit rows (which carry `approval_id` + a human
/// `label`), looks up the canonical decision on `approval_request`, and
/// emits one camelCase `RecentDecision` per **decided** request
/// (`decisionId` / `label` / `outcome` "approved"|"refused" / `decidedAt`
/// / `href`). Pending requests are not decisions and are skipped, so a
/// worker with only open asks honestly shows no decisions. No new table —
/// this is a read over existing rows.
pub(super) async fn fetch_recent_decisions(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    policy: &RedactionPolicy,
) -> ArgoResult<Vec<Value>> {
    // Spine: the worker's approval-asked audit rows (newest-first).
    let rows: Vec<(Vec<u8>, String, i64)> = sqlx::query_as(
        "SELECT id, payload_json, created_at FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? AND kind = 'approval.asked' \
         ORDER BY created_at DESC LIMIT ?",
    )
    .bind(ctx.tenant.to_string())
    .bind(ctx.worker.id.to_string())
    .bind(DECISION_LIMIT)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("recent-decisions audit scan: {e}")))?;

    let mut out = Vec::new();
    for (audit_id, payload_json, created_at) in rows {
        let payload: Value = serde_json::from_str(&payload_json).unwrap_or(Value::Null);
        let Some(approval_id) = payload.get("approval_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(decision) = lookup_decision(store, ctx.tenant, approval_id).await? else {
            // Still pending / not found — not a decision.
            continue;
        };
        let label = payload
            .get("label")
            .or_else(|| payload.get("summary"))
            .and_then(Value::as_str)
            .unwrap_or("Approval decision");
        let (label, _) = redact_text(label, policy);
        let decided_at = Utc
            .timestamp_micros(decision.decided_at_micros.unwrap_or(created_at))
            .single()
            .map_or_else(String::new, |dt| dt.to_rfc3339());
        let href = Uuid::from_slice(&audit_id)
            .map(|u| format!("/audit/action/{u}"))
            .unwrap_or_default();
        out.push(json!({
            "decisionId": approval_id,
            "label": label,
            "outcome": decision.outcome,
            "decidedAt": decided_at,
            "href": href,
        }));
    }
    Ok(out)
}

struct DecidedApproval {
    /// `"approved"` or `"refused"` — the SPA chip value.
    outcome: &'static str,
    decided_at_micros: Option<i64>,
}

/// Look up the canonical decision for `approval_id` under `tenant`.
/// Returns `None` when the request is still pending or does not exist
/// (so the caller skips it). Tenant-scoped: a cross-tenant id matches no
/// row. Maps the stored status `granted`→approved, `denied`/`expired`→
/// refused.
async fn lookup_decision(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    approval_id: &str,
) -> ArgoResult<Option<DecidedApproval>> {
    let Ok(id) = Uuid::parse_str(approval_id) else {
        return Ok(None);
    };
    let row: Option<(String, Option<i64>)> = sqlx::query_as(
        "SELECT status, decided_at FROM approval_request \
         WHERE id = ? AND tenant_id = ?",
    )
    .bind(id.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("recent-decisions approval lookup: {e}")))?;
    let Some((status, decided_at)) = row else {
        return Ok(None);
    };
    let outcome = match status.as_str() {
        "granted" => "approved",
        "denied" | "expired" => "refused",
        _ => return Ok(None), // pending — not a decision
    };
    Ok(Some(DecidedApproval {
        outcome,
        decided_at_micros: decided_at,
    }))
}

// ----- voice profile -------------------------------------------------------

pub(super) fn voice_profile_json(worker: &Worker) -> Value {
    let v = &worker.persona.voice_profile;
    json!({
        "preamble": v.preamble,
        "tone": format!("{:?}", v.tone).to_ascii_lowercase(),
        "formality": format!("{:?}", v.formality).to_ascii_lowercase(),
        "lexicon": v.lexicon,
        "forbidden_phrases": v.forbidden_phrases,
    })
}

// ----- audit ---------------------------------------------------------------

pub(super) async fn emit_audit(
    store: &Arc<SessionStore>,
    req: &str,
    ctx: &WorkerCtx,
    endpoint: &str,
    extra: Value,
) {
    let mut payload = json!({
        "endpoint": endpoint,
        "tenant_id": ctx.tenant.to_string(),
        "worker_id": ctx.worker.id.to_string(),
    });
    if let (Some(map), Some(obj)) = (payload.as_object_mut(), extra.as_object()) {
        for (k, v) in obj {
            map.insert(k.clone(), v.clone());
        }
    }
    if let Err(e) = AuditSink::new_with_tenant(Arc::clone(store), ctx.tenant.to_string())
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            principal_id_for_audit(req),
        )
        .await
    {
        tracing::warn!(target: "gateway::worker_memory", error = %e, "audit write failed");
    }
}

// ----- error envelopes -----------------------------------------------------

pub(super) fn bad_request(message: &str) -> (&'static str, String) {
    envelope("400 Bad Request", "bad_request", message)
}
pub(super) fn not_found(message: &str) -> (&'static str, String) {
    envelope("404 Not Found", "not_found", message)
}
pub(super) fn server_error(message: &str) -> (&'static str, String) {
    envelope("500 Internal Server Error", "internal_error", message)
}
fn envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

// ----- test seeding + introspection (cross-tenant negatives) ---------------

/// Seed a fact under `worker`'s namespace, bypassing cwd-tenant resolution
/// so an "alien" tenant can own it. Used by cross-tenant negative tests.
#[cfg(test)]
pub(super) async fn seed_alien_fact(
    store: &Arc<SessionStore>, worker: &Worker, text: &str,
) -> Uuid {
    let workspace = crate::home::cwd_workspace(store).await.expect("workspace");
    MemoryStore::new(Arc::clone(store))
        .append_in_namespace(
            &worker.memory_namespace, workspace, MemoryFile::Memory, text.to_string(), None,
        )
        .await.expect("seed alien fact").id
}

#[cfg(test)]
pub(super) async fn entry_count(store: &SessionStore) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM memory_entry")
        .fetch_one(store.pool()).await.expect("count")
}

#[cfg(test)]
pub(super) async fn fact_body(store: &SessionStore, fact_id: Uuid) -> String {
    sqlx::query_scalar("SELECT body FROM memory_entry WHERE id = ?")
        .bind(fact_id.as_bytes().as_slice())
        .fetch_one(store.pool()).await.expect("fetch body")
}
