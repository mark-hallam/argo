//! `/api/v1/workers/<id>/memory{,/facts/<fact_id>}` — Stream J10.
//!
//! Sensible deviations from the brief:
//!
//! - **Soft-delete primitive.** The brief specifies a `deleted_at`
//!   column on `memory_entry`, but `argo-memory` is read-only here
//!   and the schema is append-only. Soft-delete = `supersede` to a
//!   tombstone marker body ([`FACT_TOMBSTONE_MARKER`]); the audit
//!   row carries the original text. Real `deleted_at` arrives with
//!   `mission-enterprise-memory-retention`.
//! - **Active conversations / recent decisions.** v1 has no
//!   `conversation_thread` table and `audit_record` rows carry
//!   neither `counterparty_handle` nor a `decision_meta.material`
//!   marker, so both sections are stable empty arrays in v1. Shape
//!   is preserved so the frontend renders "no activity yet" until
//!   the named follow-up briefs populate the underlying signals.
//! - **PUT/DELETE routing.** No dedicated PUT/DELETE dispatchers;
//!   `routes_dispatch_post` already carries non-POST routes (DELETE
//!   for elevated-view, PATCH for retros).
//! - `namespace_id` is hidden from the `delegator` role.
use super::routes_auth_principal::principal_id_for_audit;
use crate::home::cwd_workspace;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, MemoryScope, Worker, WorkerId};
use argo_memory::{MemoryEntry, MemoryFile, MemoryStore, PrincipalOverride};
use argo_redact::{redact_text, RedactionPolicy};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

const FACT_MAX_BYTES: usize = 1000;
const ROLE_HEADER: &str = "X-Argo-Role";
const FACT_TOMBSTONE_MARKER: &str = "\u{0007}skycot-fact-tombstone\u{0007}";

/// `GET /api/v1/workers/<id>/memory`.
pub(super) async fn list_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return super::store_unavailable() };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ctx = match resolve_worker_ctx(store, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let policy = RedactionPolicy::default();
    let facts = match fetch_active_facts(store, &ctx).await {
        Ok(rs) => rs.into_iter().map(|r| fact_json(&r, &policy)).collect::<Vec<_>>(),
        Err(e) => return server_error(&format!("fact list: {e}")),
    };
    let visible = parse_role_header(req).as_deref().is_some_and(role_shows_ns);
    let body = json!({
        "facts": facts,
        // See module docstring — both arrays are intentionally empty
        // in v1 (underlying signals don't exist yet).
        "active_conversations": Vec::<Value>::new(),
        "recent_decisions": Vec::<Value>::new(),
        "voice_profile": voice_profile_json(&ctx.worker),
        "namespace_id": if visible {
            Value::String(format_namespace_id(&ctx.worker))
        } else {
            Value::Null
        },
    });
    emit_audit(store, req, &ctx, "worker.memory.viewed", json!({ "facts_returned": facts.len() }))
        .await;
    ("200 OK", body.to_string())
}

/// `POST /api/v1/workers/<id>/memory/facts`.
pub(super) async fn fact_create_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, ctx, text) = match prepare_with_text(store, worker_id_raw, req).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let workspace = match cwd_workspace(store).await {
        Ok(w) => w,
        Err(e) => return server_error(&format!("cwd_workspace: {e}")),
    };
    let memory = MemoryStore::new(Arc::clone(store));
    let entry = match memory
        .append_in_namespace(
            &ctx.worker.memory_namespace,
            workspace,
            MemoryFile::Memory,
            text.clone(),
            None,
        )
        .await
    {
        Ok(e) => e,
        Err(e) => return server_error(&format!("fact create: {e}")),
    };
    emit_audit(
        store,
        req,
        &ctx,
        "worker.memory.fact.created",
        json!({ "fact_id": entry.id.to_string(), "new_text": text }),
    )
    .await;
    let (redacted, _) = redact_text(&text, &RedactionPolicy::default());
    (
        "201 Created",
        json!({
            "fact_id": entry.id.to_string(),
            "text": redacted,
            "created_at": entry.learned_at.to_rfc3339(),
        })
        .to_string(),
    )
}

/// `PUT /api/v1/workers/<id>/memory/facts/<fact_id>`.
pub(super) async fn fact_update_response(
    worker_id_raw: &str,
    fact_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, ctx, new_text, fact_id, existing) =
        match prepare_with_existing_fact(store, worker_id_raw, fact_id_raw, req).await {
            Ok(t) => t,
            Err(r) => return r,
        };
    let old_text = existing.body.clone();
    let memory = MemoryStore::new(Arc::clone(store));
    let new_entry = match memory
        .supersede(fact_id, new_text.clone(), None, PrincipalOverride::Agent)
        .await
    {
        Ok(e) => e,
        Err(e) => return server_error(&format!("fact update: {e}")),
    };
    emit_audit(
        store,
        req,
        &ctx,
        "worker.memory.fact.edited",
        json!({
            "fact_id_original": fact_id.to_string(),
            "fact_id_new_head": new_entry.id.to_string(),
            "old_text": old_text,
            "new_text": new_text,
        }),
    )
    .await;
    let (redacted, _) = redact_text(&new_text, &RedactionPolicy::default());
    (
        "200 OK",
        json!({
            "fact_id": fact_id.to_string(),
            "text": redacted,
            "edited_at": new_entry.learned_at.to_rfc3339(),
        })
        .to_string(),
    )
}

/// `DELETE /api/v1/workers/<id>/memory/facts/<fact_id>`.
pub(super) async fn fact_delete_response(
    worker_id_raw: &str,
    fact_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return super::store_unavailable() };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let fact_id = match parse_fact_id(fact_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ctx = match resolve_worker_ctx(store, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let existing = match fetch_owned_active_fact(store, &ctx, fact_id).await {
        Ok(Some(e)) => e,
        Ok(None) => return not_found(&format!("fact {fact_id} not found")),
        Err(e) => return server_error(&format!("fact lookup: {e}")),
    };
    let old_text = existing.body.clone();
    if let Err(e) = MemoryStore::new(Arc::clone(store))
        .supersede(fact_id, FACT_TOMBSTONE_MARKER.into(), None, PrincipalOverride::Agent)
        .await
    {
        return server_error(&format!("fact delete: {e}"));
    }
    emit_audit(
        store,
        req,
        &ctx,
        "worker.memory.fact.deleted",
        json!({ "fact_id": fact_id.to_string(), "old_text": old_text }),
    )
    .await;
    ("204 No Content", String::new())
}

// ----- prepare combinators -------------------------------------------------

async fn prepare_with_text<'s>(
    store: Option<&'s Arc<SessionStore>>,
    worker_id_raw: &str,
    req: &str,
) -> Result<(&'s Arc<SessionStore>, WorkerCtx, String), (&'static str, String)> {
    let store = store.ok_or_else(super::store_unavailable)?;
    let worker_id = parse_worker_id(worker_id_raw)?;
    let text = parse_text_body(req)?;
    let ctx = resolve_worker_ctx(store, worker_id).await?;
    Ok((store, ctx, text))
}

#[allow(clippy::type_complexity)]
async fn prepare_with_existing_fact<'s>(
    store: Option<&'s Arc<SessionStore>>,
    worker_id_raw: &str,
    fact_id_raw: &str,
    req: &str,
) -> Result<
    (&'s Arc<SessionStore>, WorkerCtx, String, Uuid, MemoryEntry),
    (&'static str, String),
> {
    let store = store.ok_or_else(super::store_unavailable)?;
    let worker_id = parse_worker_id(worker_id_raw)?;
    let fact_id = parse_fact_id(fact_id_raw)?;
    let text = parse_text_body(req)?;
    let ctx = resolve_worker_ctx(store, worker_id).await?;
    let existing = match fetch_owned_active_fact(store, &ctx, fact_id).await {
        Ok(Some(e)) => e,
        Ok(None) => return Err(not_found(&format!("fact {fact_id} not found"))),
        Err(e) => return Err(server_error(&format!("fact lookup: {e}"))),
    };
    Ok((store, ctx, text, fact_id, existing))
}

// ----- lookup + validation -------------------------------------------------

struct WorkerCtx {
    tenant: TenantId,
    worker: Worker,
}

fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    t.parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{t}': {e}")))
}

fn parse_fact_id(raw: &str) -> Result<Uuid, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    Uuid::parse_str(t).map_err(|e| bad_request(&format!("invalid fact_id '{t}': {e}")))
}

fn parse_text_body(req: &str) -> Result<String, (&'static str, String)> {
    let parsed: Value = serde_json::from_str(super::body_from_request(req))
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
        return Err(bad_request(&format!("text exceeds {FACT_MAX_BYTES}-byte limit")));
    }
    if raw.contains(FACT_TOMBSTONE_MARKER) {
        return Err(bad_request("text contains reserved tombstone marker"));
    }
    Ok(raw)
}

async fn resolve_worker_ctx(
    store: &Arc<SessionStore>,
    worker_id: WorkerId,
) -> Result<WorkerCtx, (&'static str, String)> {
    let tenant = resolve_workspace_tenant(store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    match IdentityStore::new(store).get_worker(tenant, worker_id).await {
        Ok(Some(worker)) => Ok(WorkerCtx { tenant, worker }),
        // Cross-tenant / cross-worker / unknown all surface as 404 —
        // per acceptance criteria, don't leak existence.
        Ok(None) => Err(not_found(&format!("worker {worker_id} not found"))),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn parse_role_header(req: &str) -> Option<String> {
    let headers = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    headers.lines().skip(1).find_map(|line| {
        let (n, v) = line.split_once(':')?;
        n.eq_ignore_ascii_case(ROLE_HEADER).then(|| v.trim().to_ascii_lowercase())
    })
}

fn role_shows_ns(role: &str) -> bool {
    matches!(role, "operator" | "auditor" | "compliance-officer" | "compliance_officer")
}

fn format_namespace_id(worker: &Worker) -> String {
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

struct FactRow {
    id: Uuid,
    body: String,
    learned_at: DateTime<Utc>,
}

async fn fetch_active_facts(
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
            Ok(FactRow { id, body, learned_at })
        })
        .collect()
}

/// `Ok(None)` for unknown / cross-tenant / cross-worker / tombstoned
/// ids so the caller surfaces one uniform 404.
async fn fetch_owned_active_fact(
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
    let owner: Option<(Vec<u8>, Vec<u8>, String)> = sqlx::query_as(
        "SELECT tenant_id, worker_id, memory_scope FROM memory_entry WHERE id = ?",
    )
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

fn fact_json(row: &FactRow, policy: &RedactionPolicy) -> Value {
    let (text, _) = redact_text(&row.body, policy);
    json!({
        "fact_id": row.id.to_string(),
        "text": text,
        "created_at": row.learned_at.to_rfc3339(),
    })
}

// ----- voice profile -------------------------------------------------------

fn voice_profile_json(worker: &Worker) -> Value {
    let v = &worker.persona.voice_profile;
    json!({
        "preamble": v.preamble,
        "tone": format!("{:?}", v.tone).to_ascii_lowercase(),
        "formality": format!("{:?}", v.formality).to_ascii_lowercase(),
        "lexicon": v.lexicon,
        "forbidden_phrases": v.forbidden_phrases,
    })
}

async fn emit_audit(
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
    if let Err(e) = AuditSink::new(Arc::clone(store))
        .write(None, AuditKind::HttpRequest, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(target: "gateway::worker_memory", error = %e, "audit write failed");
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    envelope("400 Bad Request", "bad_request", message)
}
fn not_found(message: &str) -> (&'static str, String) {
    envelope("404 Not Found", "not_found", message)
}
fn server_error(message: &str) -> (&'static str, String) {
    envelope("500 Internal Server Error", "internal_error", message)
}
fn envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_worker_memory_tests.rs"]
mod tests;
