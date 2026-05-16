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
use argo_memory::{MemoryEntry, MemoryFile, MemoryStore, PrincipalOverride};
use argo_redact::{redact_text, RedactionPolicy};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

#[path = "routes_worker_memory_helpers.rs"]
mod helpers;

use helpers::{
    emit_audit, fact_json, fetch_active_facts, fetch_owned_active_fact, format_namespace_id,
    not_found, parse_fact_id, parse_role_header, parse_text_body, parse_worker_id,
    resolve_worker_ctx, role_shows_ns, server_error, voice_profile_json, WorkerCtx,
};

const FACT_MAX_BYTES: usize = 1000;
const ROLE_HEADER: &str = "X-Argo-Role";
const FACT_TOMBSTONE_MARKER: &str = "\u{0007}skycot-fact-tombstone\u{0007}";

/// `GET /api/v1/workers/<id>/memory`.
pub(super) async fn list_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
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
        Ok(rs) => rs
            .into_iter()
            .map(|r| fact_json(&r, &policy))
            .collect::<Vec<_>>(),
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
    emit_audit(
        store,
        req,
        &ctx,
        "worker.memory.viewed",
        json!({ "facts_returned": facts.len() }),
    )
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
    let workspace = match crate::home::cwd_workspace(store).await {
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
    let Some(store) = store else {
        return super::store_unavailable();
    };
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
        .supersede(
            fact_id,
            FACT_TOMBSTONE_MARKER.into(),
            None,
            PrincipalOverride::Agent,
        )
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
) -> Result<(&'s Arc<SessionStore>, WorkerCtx, String, Uuid, MemoryEntry), (&'static str, String)> {
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

#[cfg(test)]
#[path = "routes_worker_memory_tests.rs"]
mod tests;
