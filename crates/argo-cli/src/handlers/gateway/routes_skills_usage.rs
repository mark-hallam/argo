//! `GET /api/v1/skills/<id-or-name>/usage` — per-skill provenance +
//! workers-using + global adoption.
//!
//! Extracted from `routes_skills_list_get` to keep that file under the
//! 500-LOC ceiling once the usage route grew a name→id fallback
//! (Frontier-B backend-route batch). The route accepts EITHER the
//! catalog UUID (the SPA's skill-detail page keys on it) OR the skill
//! name (the marketplace card / older SPA paths pass the name), so a
//! name-style segment resolves instead of `404 "invalid skill id"`.
//!
//! Tenant resolution + the `header_value` helper are reused from the
//! sibling `routes_skills_list_get` module (both `pub(super)`).

use super::routes_skills_list_get::request_tenant_id;
use argo_identity::IdentityStore;
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// `GET /api/v1/skills/<id-or-name>/usage`. Accepts either the catalog
/// UUID or the skill name. Same envelope the bare `routes_skills` usage
/// route returns. Real data: a skill row lookup (by id or name) plus the
/// `worker_skill` adoption queries — the adoption queries always run
/// against the resolved UUID, so a name-keyed lookup still produces
/// id-based adoption numbers.
pub(super) async fn skill_usage_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    id_str: &str,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant_str = match request_tenant_id(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    // Resolve the skill row by UUID first; if `id_str` isn't a UUID,
    // fall back to a tenant-scoped name lookup so callers passing the
    // skill name (not the catalog id) still resolve instead of 404.
    let (uuid, skill_name, source) = match resolve_skill_for_usage(store, &tenant_str, id_str).await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let Ok(tenant_uuid) = Uuid::parse_str(&tenant_str) else {
        return (
            "500 Internal Server Error",
            json!({ "error": format!("tenant id not a uuid: '{tenant_str}'") }).to_string(),
        );
    };
    let identity = IdentityStore::new(store);
    let id_string = uuid.to_string();
    let workers = match identity
        .workers_with_skill_learned_at(TenantId(tenant_uuid), &id_string)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json!({ "error": format!("workers_with_skill: {e}") }).to_string(),
            )
        }
    };
    let adoption = match identity.count_workers_with_skill_global(&id_string).await {
        Ok(c) => c,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json!({ "error": format!("adoption count: {e}") }).to_string(),
            )
        }
    };
    let workers_json: Vec<Value> = workers
        .iter()
        .map(|(worker, learned_at)| {
            json!({
                "worker_id": worker.id.to_string(),
                "name": worker.persona.name,
                "learned_at": learned_at,
            })
        })
        .collect();
    let payload = json!({
        "skill_id": id_string,
        "skill_name": skill_name,
        "trust_grade": TrustGrade::from_source(&source),
        "author_label": author_label_for_source(&source),
        "workers_using": workers_json,
        "workers_using_count": workers.len(),
        "adoption_count": adoption,
    });
    ("200 OK", payload.to_string())
}

/// Resolve a skill's `(uuid, name, source)` for the usage route,
/// accepting either the catalog UUID or the skill name. Scoped to the
/// caller's tenant + active, non-superseded rows. Returns the canonical
/// `404 Not Found` envelope when no matching skill exists, or `500` on a
/// DB error.
async fn resolve_skill_for_usage(
    store: &Arc<SessionStore>,
    tenant_str: &str,
    id_or_name: &str,
) -> Result<(Uuid, String, String), (&'static str, String)> {
    let row = if let Ok(uuid) = Uuid::parse_str(id_or_name) {
        let id_bytes = uuid.as_bytes().to_vec();
        sqlx::query(
            "SELECT s.id, s.name, s.source FROM skill s \
             WHERE s.id = ? AND s.tenant_id = ? \
               AND s.superseded_by IS NULL AND s.lifecycle_state = 'active' LIMIT 1",
        )
        .bind(&id_bytes)
        .bind(tenant_str)
        .fetch_optional(store.pool())
        .await
    } else {
        sqlx::query(
            "SELECT s.id, s.name, s.source FROM skill s \
             WHERE s.name = ? AND s.tenant_id = ? \
               AND s.superseded_by IS NULL AND s.lifecycle_state = 'active' \
             ORDER BY s.created_at DESC LIMIT 1",
        )
        .bind(id_or_name)
        .bind(tenant_str)
        .fetch_optional(store.pool())
        .await
    };
    match row {
        Ok(Some(r)) => {
            let id_blob: Vec<u8> = r.try_get("id").unwrap_or_default();
            let Some(uuid) = uuid_from_blob(&id_blob) else {
                return Err((
                    "500 Internal Server Error",
                    json!({ "error": "skill id column not a 16-byte uuid" }).to_string(),
                ));
            };
            Ok((
                uuid,
                r.try_get("name").unwrap_or_default(),
                r.try_get("source").unwrap_or_default(),
            ))
        }
        Ok(None) => Err((
            "404 Not Found",
            json!({ "error": format!("no active skill '{id_or_name}' in this tenant") })
                .to_string(),
        )),
        Err(e) => Err((
            "500 Internal Server Error",
            json!({ "error": format!("skill lookup: {e}") }).to_string(),
        )),
    }
}

/// Decode a 16-byte BLOB id column into a `Uuid`. Returns `None` for any
/// other length so a malformed row surfaces a 500 rather than panicking.
fn uuid_from_blob(blob: &[u8]) -> Option<Uuid> {
    if blob.len() == 16 {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(blob);
        Some(Uuid::from_bytes(arr))
    } else {
        None
    }
}

/// Map a skill's `provenance.source` to the operator-facing authorship
/// label — anonymized, no per-tenant identity. Mirrors the helper in
/// `routes_skills` (kept local so this module stays self-contained).
fn author_label_for_source(source: &str) -> &'static str {
    if source == "builtin"
        || source == "verticals"
        || source.starts_with("published:verified:market")
        || source.starts_with("published:community:market")
    {
        "Authored by SKYCOT"
    } else {
        "Authored by your org"
    }
}

// Integration coverage for this route lives in
// `routes_skills_list_get_tests.rs` (the `usage_*` tests), which drive
// the full `routes_skills_list_get::dispatch_get` path that lands here —
// so the name→id fallback, by-id resolution, and 404 miss are all
// exercised end-to-end through the dispatcher.
