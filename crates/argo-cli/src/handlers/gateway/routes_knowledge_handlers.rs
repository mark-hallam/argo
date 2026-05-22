//! Per-route handler bodies for `routes_knowledge`. Split out from
//! the parent module to keep both files under the 500-LOC ceiling.
//!
//! Each handler does three things:
//!
//! 1. Resolves the tenant-scoped substrate call (read or write).
//! 2. For mutations, emits one [`argo_audit::AuditKind`] row through
//!    [`AuditSink::new_with_tenant`] — the canonical pattern from
//!    `routes_workflows_handlers.rs`.
//! 3. Serialises the result through `doc_json` / `revision_json` so
//!    the wire shape stays stable across handlers.
//!
//! ## Diff in the update payload
//!
//! `WorkplaceKnowledgeDocUpdated` rows include a `diff` field per the
//! design doc §7 acceptance. We compute a tiny prefix-diff (first 500
//! characters of a line-pair preview) — the full prior body is
//! recoverable from `workplace_knowledge_revision.body_markdown`, so
//! the audit row's diff is for "what changed at a glance" rather than
//! a true unified diff. Keeps the audit row size bounded.

use super::routes_knowledge::{bad_request, conflict, internal, not_found};
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{ArgoError, TenantId};
use argo_workplace_knowledge::doc::{self, KnowledgeCategory, WorkplaceKnowledgeDoc};
use argo_workplace_knowledge::revision::{self, WorkplaceKnowledgeRevision};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// Truncation ceiling for the diff snippet baked into the audit row.
/// Keeps `audit_record.payload_json` bounded so large body swaps
/// don't bloat the chain. The full prior body lives on the revision
/// row for any deeper diff-viewer rendering.
const DIFF_MAX_CHARS: usize = 500;

// ── Read handlers ────────────────────────────────────────────────

pub(super) async fn handle_list(
    store: &SessionStore,
    tenant: TenantId,
    category_filter: Option<&str>,
) -> (&'static str, String) {
    match doc::list(store.pool(), &tenant).await {
        Ok(rows) => {
            let category = match category_filter {
                Some(c) => match KnowledgeCategory::from_db_str(c) {
                    Some(cat) => Some(cat),
                    None => return bad_request(&format!("unknown category '{c}'")),
                },
                None => None,
            };
            let items: Vec<Value> = rows
                .iter()
                .filter(|d| category.is_none_or(|c| d.category == c))
                .map(doc_json)
                .collect();
            let body = json!({
                "count": items.len(),
                "items": items,
            });
            ("200 OK", body.to_string())
        }
        Err(e) => internal(&format!("knowledge list: {e}")),
    }
}

pub(super) async fn handle_get(
    store: &SessionStore,
    tenant: TenantId,
    slug: &str,
) -> (&'static str, String) {
    match doc::get(store.pool(), &tenant, slug).await {
        Ok(Some(d)) => ("200 OK", doc_json(&d).to_string()),
        Ok(None) => not_found(&format!("knowledge slug '{slug}' not found")),
        Err(e) => internal(&format!("knowledge get: {e}")),
    }
}

pub(super) async fn handle_revisions(
    store: &SessionStore,
    tenant: TenantId,
    slug: &str,
) -> (&'static str, String) {
    // Probe the head to translate `(tenant, slug) → doc_id`. For a
    // soft-deleted doc the head is gone and the revision table does
    // not currently carry a `slug` column, so the slug→history hop
    // returns 404 in that case. Sensible deviation from the brief's
    // note about deleted docs keeping a walkable timeline: a slug-
    // indexed revision lookup belongs in brief 1's substrate domain;
    // we don't widen this brief's may-touch list to add a column.
    let doc_id = match doc::get(store.pool(), &tenant, slug).await {
        Ok(Some(d)) => d.id,
        Ok(None) => return not_found(&format!("knowledge slug '{slug}' not found")),
        Err(e) => return internal(&format!("knowledge head probe: {e}")),
    };
    match revision::list_for_doc(store.pool(), &tenant, &doc_id).await {
        Ok(rows) => {
            let items: Vec<Value> = rows.iter().map(revision_json).collect();
            let body = json!({
                "slug": slug,
                "count": items.len(),
                "items": items,
            });
            ("200 OK", body.to_string())
        }
        Err(e) => internal(&format!("knowledge revisions: {e}")),
    }
}

// ── Mutating handlers ────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateBody {
    slug: String,
    title: String,
    category: String,
    body_markdown: String,
}

#[derive(Deserialize)]
struct UpdateBody {
    body_markdown: Option<String>,
    edit_reason: Option<String>,
}

pub(super) async fn handle_create(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    body: &str,
    actor: &str,
) -> (&'static str, String) {
    let parsed: CreateBody = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse create body: {e}")),
    };
    let category = match KnowledgeCategory::from_db_str(&parsed.category) {
        Some(c) => c,
        None => return bad_request(&format!("unknown category '{}'", parsed.category)),
    };
    let created_by = if actor.is_empty() { "anonymous" } else { actor };
    match doc::create(
        store.pool(),
        &tenant,
        &parsed.slug,
        &parsed.title,
        category,
        &parsed.body_markdown,
        created_by,
    )
    .await
    {
        Ok(d) => {
            emit_audit(
                store,
                tenant,
                AuditKind::WorkplaceKnowledgeDocCreated,
                actor,
                json!({
                    "slug": d.slug,
                    "title": d.title,
                    "category": d.category.as_db_str(),
                    "version": d.version,
                }),
            )
            .await;
            ("201 Created", doc_json(&d).to_string())
        }
        Err(ArgoError::Schema(msg)) => bad_request(&msg),
        Err(ArgoError::Provider(msg)) if msg.contains("UNIQUE") => conflict(&format!(
            "knowledge slug '{}' already exists for tenant",
            parsed.slug
        )),
        Err(e) => internal(&format!("knowledge create: {e}")),
    }
}

pub(super) async fn handle_update(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    slug: &str,
    body: &str,
    actor: &str,
) -> (&'static str, String) {
    let parsed: UpdateBody = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse update body: {e}")),
    };
    let new_body = match parsed.body_markdown.as_deref() {
        Some(s) => s,
        None => return bad_request("update body must include `body_markdown`"),
    };
    // Probe the prior body so the audit row's `diff` field has both
    // sides without an extra DB round-trip after the substrate
    // returns the refreshed head.
    let prior = match doc::get(store.pool(), &tenant, slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return not_found(&format!("knowledge slug '{slug}' not found")),
        Err(e) => return internal(&format!("knowledge update probe: {e}")),
    };
    let updated_by = if actor.is_empty() { "anonymous" } else { actor };
    match doc::update(
        store.pool(),
        &tenant,
        slug,
        new_body,
        updated_by,
        parsed.edit_reason.as_deref(),
    )
    .await
    {
        Ok(d) => {
            let diff = short_diff(&prior.body_markdown, &d.body_markdown);
            emit_audit(
                store,
                tenant,
                AuditKind::WorkplaceKnowledgeDocUpdated,
                actor,
                json!({
                    "slug": d.slug,
                    "version": d.version,
                    "edit_reason": parsed.edit_reason,
                    "diff": diff,
                }),
            )
            .await;
            ("200 OK", doc_json(&d).to_string())
        }
        Err(ArgoError::NotFound(msg)) => not_found(&msg),
        Err(ArgoError::Schema(msg)) => bad_request(&msg),
        Err(e) => internal(&format!("knowledge update: {e}")),
    }
}

pub(super) async fn handle_delete(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    slug: &str,
    actor: &str,
) -> (&'static str, String) {
    match doc::delete(store.pool(), &tenant, slug).await {
        Ok(true) => {
            emit_audit(
                store,
                tenant,
                AuditKind::WorkplaceKnowledgeDocDeleted,
                actor,
                json!({ "slug": slug }),
            )
            .await;
            ("204 No Content", String::new())
        }
        Ok(false) => not_found(&format!("knowledge slug '{slug}' not found")),
        Err(e) => internal(&format!("knowledge delete: {e}")),
    }
}

// ── Audit emission + JSON shape ─────────────────────────────────

async fn emit_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    kind: AuditKind,
    actor: &str,
    extra: Value,
) {
    let mut payload = json!({
        "actor": actor,
        "tenant_id": tenant.to_string(),
    });
    if let (Some(obj), Value::Object(extra_obj)) = (payload.as_object_mut(), extra) {
        for (k, v) in extra_obj {
            obj.insert(k, v);
        }
    }
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let principal = if actor.is_empty() {
        None
    } else {
        Some(actor.to_string())
    };
    if let Err(e) = sink.write(None, kind, payload, principal).await {
        tracing::warn!(
            target: "gateway::routes_knowledge",
            error = %e,
            kind = %kind.label(),
            "workplace_knowledge audit write failed",
        );
    }
}

pub(super) fn doc_json(d: &WorkplaceKnowledgeDoc) -> Value {
    json!({
        "id": d.id,
        "tenant_id": d.tenant_id,
        "slug": d.slug,
        "title": d.title,
        "category": d.category.as_db_str(),
        "body_markdown": d.body_markdown,
        "version": d.version,
        "parent_revision_id": d.parent_revision_id,
        "created_at_micros": d.created_at_micros,
        "created_by": d.created_by,
        "updated_at_micros": d.updated_at_micros,
        "updated_by": d.updated_by,
    })
}

pub(super) fn revision_json(r: &WorkplaceKnowledgeRevision) -> Value {
    json!({
        "id": r.id,
        "doc_id": r.doc_id,
        "tenant_id": r.tenant_id,
        "version": r.version,
        "body_markdown": r.body_markdown,
        "diff_from_prior": r.diff_from_prior,
        "edited_at_micros": r.edited_at_micros,
        "edited_by": r.edited_by,
        "edit_reason": r.edit_reason,
    })
}

/// Compact diff snippet for the audit row's `diff` field. We don't
/// ship a true unified diff (the audit chain has no need for that
/// fidelity and the full prior body lives on the revision row).
/// Instead emit the first `DIFF_MAX_CHARS` of a "before → after"
/// preview so the chain stays bounded and the SPA can render a quick
/// "what changed?" affordance from the audit feed alone.
pub(super) fn short_diff(prior: &str, new: &str) -> String {
    let prior_preview = take_chars(prior, DIFF_MAX_CHARS / 2);
    let new_preview = take_chars(new, DIFF_MAX_CHARS / 2);
    format!("- {prior_preview}\n+ {new_preview}")
}

fn take_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
