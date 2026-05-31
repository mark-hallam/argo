//! Seeded-approval bridge: surface `approval_request` table rows on
//! `GET /api/v1/approvals`.
//!
//! The legacy approvals list (`super::routes_approvals_json`) reads
//! `worker_action` memory rows — that's the path real agent runs use.
//! `argo demo seed` writes to a parallel store, the first-class
//! `approval_request` table (`crates/argo-store/src/approvals.rs`),
//! which the legacy list never queries. Before this module the demo
//! `/approvals` page rendered empty despite 8 seeded rows
//! (`fix-approvals-list-tenant-scope-empty-despite-seeded-rows`).
//!
//! The two stores remain disjoint at the data level; this module is
//! the read-side bridge that surfaces both paths through the one
//! gateway endpoint. Decide / undo continues to flow through the
//! legacy worker-action transitions — out of scope for this fix.
//!
//! Tenant resolution mirrors the audit feed: cwd-bound workspace
//! tenant via `SessionStore::ensure_workspace`, with `X-Tenant-Id` /
//! `X-Argo-Tenant-Id` header override. The cross-tenant invisibility
//! property held by `claim_6_cross_tenant_approval_select_no_existence_leak`
//! stays closed: the SELECT below binds the resolved tenant_id as a
//! BLOB to match how the seed writes it.
use argo_identity::{IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::home::{db_path, ensure_dir, resolve_home};

/// One seeded approval, already shaped as the SPA `ApprovalRow`
/// envelope. Carries `received_at` + `id` alongside the JSON body so
/// the caller can merge with memory-row results, sort, and cursor-
/// paginate uniformly.
pub(super) struct SeededRow {
    pub received_at: String,
    pub id: String,
    pub json: Value,
}

/// Fetch every `approval_request` row visible to the caller's tenant,
/// enriched with the matching `approval.asked` audit payload (channel
/// / worker_id / summary) and worker persona name. Returns an empty
/// list on any resolver miss — never an error from the caller's view —
/// because the legacy memory-row path is the primary list source and
/// this bridge is purely additive.
pub(super) async fn fetch_seeded_approval_rows(req: &str) -> Vec<SeededRow> {
    let Some((store, tenant_id, tenant_uuid)) = open_store_and_tenant(req).await else {
        return Vec::new();
    };
    let approval_rows = match approval_request_rows(store.pool(), &tenant_uuid).await {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };
    if approval_rows.is_empty() {
        return Vec::new();
    }
    let ask_payloads = approval_asked_payloads(store.pool(), &tenant_id)
        .await
        .unwrap_or_default();
    let identities = IdentityStore::new(store.as_ref());
    let tenant_t = TenantId(tenant_uuid);
    let mut out = Vec::with_capacity(approval_rows.len());
    for row in approval_rows {
        let ask = ask_payloads.get(&row.id);
        let worker = resolve_worker_name(&identities, tenant_t, ask).await;
        out.push(build_row(&row, ask, worker));
    }
    out
}

struct ApprovalRow {
    id: String,
    requested_at_micros: i64,
    status: String,
    /// The `requested_by` column — the teammate that filed the draft
    /// (e.g. `"Marcus"` for marketing drafts). Used as the displayed
    /// `worker.name` when no `approval.asked`-resolved worker persona is
    /// available. Empty string for rows that never set it.
    requested_by: String,
    draft_content: Option<String>,
}

async fn approval_request_rows(
    pool: &sqlx::SqlitePool,
    tenant_uuid: &Uuid,
) -> Result<Vec<ApprovalRow>, sqlx::Error> {
    // approval_request stores tenant_id as 16-byte BLOB (see
    // crates/argo-store/src/approvals.rs::create_request). Bind the
    // resolved tenant the same way; binding as TEXT would never match.
    let raw = sqlx::query(
        "SELECT id, requested_at, status, requested_by, draft_content \
         FROM approval_request \
         WHERE tenant_id = ? \
         ORDER BY requested_at DESC",
    )
    .bind(tenant_uuid.as_bytes().as_slice())
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(raw.len());
    for r in raw {
        let id_bytes: Vec<u8> = r.try_get("id")?;
        let Ok(id) = Uuid::from_slice(&id_bytes) else {
            continue;
        };
        out.push(ApprovalRow {
            id: id.to_string(),
            requested_at_micros: r.try_get("requested_at")?,
            status: r.try_get("status")?,
            requested_by: r.try_get("requested_by").unwrap_or_default(),
            draft_content: r.try_get("draft_content").ok(),
        });
    }
    Ok(out)
}

async fn approval_asked_payloads(
    pool: &sqlx::SqlitePool,
    tenant_id_text: &str,
) -> Result<HashMap<String, Value>, sqlx::Error> {
    // audit_record.tenant_id is TEXT (UUID string) per v61. Match the
    // text form the AuditSink writes via `to_string()`.
    let raw: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'approval.asked' AND tenant_id = ?",
    )
    .bind(tenant_id_text)
    .fetch_all(pool)
    .await?;
    let mut out = HashMap::with_capacity(raw.len());
    for (payload_json,) in raw {
        let Ok(v) = serde_json::from_str::<Value>(&payload_json) else {
            continue;
        };
        if let Some(approval_id) = v.get("approval_id").and_then(Value::as_str) {
            out.insert(approval_id.to_string(), v);
        }
    }
    Ok(out)
}

async fn resolve_worker_name(
    identities: &IdentityStore<'_>,
    tenant: TenantId,
    ask: Option<&Value>,
) -> Option<(WorkerId, String)> {
    let worker_id_str = ask?.get("worker_id").and_then(Value::as_str)?;
    let worker_uuid = Uuid::parse_str(worker_id_str).ok()?;
    let worker_id = WorkerId(worker_uuid);
    let worker = identities.get_worker(tenant, worker_id).await.ok()??;
    Some((worker_id, worker.persona.name))
}

fn build_row(row: &ApprovalRow, ask: Option<&Value>, worker: Option<(WorkerId, String)>) -> SeededRow {
    let received_at = chrono::DateTime::<chrono::Utc>::from_timestamp_micros(row.requested_at_micros)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();
    // Displayed name precedence: a resolved worker persona (when the
    // `approval.asked` payload carried a worker_id) wins; otherwise the
    // `approval_request.requested_by` column (e.g. "Marcus" for a
    // marketing draft); the "Pending teammate" placeholder is used ONLY
    // when neither is available. The worker id stays "unknown" unless a
    // real persona was resolved — `requested_by` is a display label, not
    // a WorkerId.
    let (worker_id_str, worker_name) = match worker {
        Some((id, name)) => (id.to_string(), name),
        None => {
            let name = match row.requested_by.trim() {
                "" => "Pending teammate".to_string(),
                by => by.to_string(),
            };
            ("unknown".to_string(), name)
        }
    };
    let verb = approval_verb(ask);
    let summary = ask
        .and_then(|v| v.get("summary"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| row.draft_content.clone())
        .unwrap_or_default();
    let channel = ask
        .and_then(|v| v.get("channel"))
        .and_then(Value::as_str)
        .unwrap_or("web");
    let decision = match row.status.as_str() {
        "approved" => Some("approve"),
        "denied" | "refused" | "expired" => Some("refuse"),
        _ => None,
    };
    let json = json!({
        "id": row.id,
        "worker": {
            "id": worker_id_str,
            "name": worker_name,
            "avatar_url": Value::Null,
        },
        "verb": verb,
        "detail": summary,
        "consequence_if_yes": "",
        "consequence_if_no": "",
        "reason": Value::Null,
        "draft_content": row.draft_content.clone().map(Value::String).unwrap_or(Value::Null),
        "channel": channel,
        "received_at": received_at.clone(),
        "decision": decision.map(|d| Value::String(d.to_string())).unwrap_or(Value::Null),
        "risk_level": "medium",
    });
    SeededRow {
        received_at,
        id: row.id.clone(),
        json,
    }
}

/// Build the card's `verb` phrase from the `approval.asked` payload.
///
/// Precedence:
/// 1. An explicit `label` field on the ask payload (`"wants to {label}"`).
/// 2. A real verb derived from the structured `action` field — e.g.
///    `"marketing.response_drafted"` → `"wants to draft a marketing
///    response"`, so Marcus's card reads grammatically rather than the
///    old ungrammatical `"wants to an action"`.
/// 3. The grammatical default `"wants to take an action"` when neither
///    is present.
fn approval_verb(ask: Option<&Value>) -> String {
    if let Some(label) = ask
        .and_then(|v| v.get("label"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return format!("wants to {label}");
    }
    if let Some(action) = ask
        .and_then(|v| v.get("action"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return verb_from_action(action);
    }
    "wants to take an action".to_string()
}

/// Turn a dotted, snake-cased `action` slug into a human verb phrase.
///
/// `"marketing.response_drafted"` → `"wants to draft a marketing
/// response"`; an unrecognised slug degrades to a readable
/// `"wants to {slug-with-spaces}"` rather than the ungrammatical
/// default.
fn verb_from_action(action: &str) -> String {
    match action {
        "marketing.response_drafted" => "wants to draft a marketing response".to_string(),
        other => {
            let words = other.replace(['.', '_'], " ");
            format!("wants to {}", words.trim())
        }
    }
}

async fn open_store_and_tenant(req: &str) -> Option<(Arc<SessionStore>, String, Uuid)> {
    let cwd = std::env::current_dir().ok()?;
    let home = resolve_home(&cwd).ok()?;
    ensure_dir(&home).ok()?;
    let store = SessionStore::open(&db_path(&home)).await.ok()?;
    store.migrate().await.ok()?;
    let store = Arc::new(store);
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (workspace_tenant, _ws) = store.ensure_workspace(name, &root).await.ok()?;
    // Tenant resolution mirrors the audit feed contract: explicit
    // `X-Tenant-Id` / `X-Argo-Tenant-Id` header overrides the
    // cwd-bound workspace default. The config default is intentionally
    // NOT consulted here — the seed writes to either the cwd workspace
    // (most common) or an explicit `--tenant <T>` (header-overridden).
    let tenant_text = header_tenant_override(req).unwrap_or_else(|| workspace_tenant.to_string());
    let tenant_uuid = Uuid::parse_str(&tenant_text).ok()?;
    Some((store, tenant_text, tenant_uuid))
}

/// Read the canonical tenant override headers off a raw request. Falls
/// back through `?tenant=` query param → `X-Tenant-Id` → legacy
/// `X-Argo-Tenant-Id` (mirroring `rate_limit::extract_tenant`'s
/// precedence chain, minus its config-default fallback).
fn header_tenant_override(req: &str) -> Option<String> {
    let target = req
        .split_once(' ')
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .unwrap_or("");
    if let Some((_, qs)) = target.split_once('?') {
        for kv in qs.split('&') {
            if let Some(("tenant", value)) = kv.split_once('=') {
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(h, _)| h);
    for line in headers.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if name.eq_ignore_ascii_case("x-tenant-id")
            || name.eq_ignore_ascii_case("x-argo-tenant-id")
        {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
#[path = "routes_approvals_json_seeded_tests.rs"]
mod tests;
