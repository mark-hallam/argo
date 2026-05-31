//! `GET /api/v1/skills`, `GET /api/v1/skills/marketplace`,
//! `GET /api/v1/skill-library/list` — tenant-scoped + cross-tenant
//! skill catalog endpoints.
//!
//! The existing `/skills` route in [`super::routes_skills`] is bare
//! (no `/api/v1/` prefix) and returns a `LayeredLookup` envelope
//! keyed on `risk`. The SPA `/skills` page expects the canonical
//! `/api/v1/*` shapes documented in
//! `fix-gateway-workflows-and-skills-marketplace-endpoints-404`,
//! so this module mounts:
//!
//! * `GET /api/v1/skills` — installed skills for the caller's
//!   tenant (filtered by `skill.tenant_id`).
//! * `GET /api/v1/skills/marketplace` — cross-tenant published
//!   catalog (rows whose provenance.source starts with
//!   `published:`); the un-installed view a prospect browses.
//! * `GET /api/v1/skill-library/list` — alias of the marketplace
//!   route, kept for SPA fallback paths.
//!
//! Envelope: `{ "skills": [{ id, name, description, layer, version,
//! trust_grade, provenance_source, tags, ... }], "total_count": N }`.
//! CORS preflight is handled in [`super::routes::dispatch_route`].
use argo_identity::IdentityStore;
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Datelike;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

const SKILLS_PATH: &str = "/api/v1/skills";
const MARKETPLACE_PATH: &str = "/api/v1/skills/marketplace";
const SKILL_LIBRARY_LIST_PATH: &str = "/api/v1/skill-library/list";
const LEARNED_THIS_MONTH_PATH: &str = "/api/v1/skills/learned-this-month";

/// Top-level dispatcher. Returns `Some(resp)` for any path served by
/// this module, `None` to let the caller fall through. Exact-match
/// routes (marketplace / learned-this-month) are checked BEFORE the
/// `/api/v1/skills/<id>` detail catch-all so those literals aren't
/// mistaken for a skill id.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !req.starts_with("GET ") {
        return None;
    }
    let after = req.strip_prefix("GET ").unwrap_or(req);
    let request_path = after.split_whitespace().next().unwrap_or("");
    let bare = request_path.split('?').next().unwrap_or(request_path);

    if bare == MARKETPLACE_PATH || bare == SKILL_LIBRARY_LIST_PATH {
        return Some(marketplace_response(store).await);
    }
    if bare == LEARNED_THIS_MONTH_PATH {
        return Some(learned_this_month_response(req, store).await);
    }
    if bare == SKILLS_PATH {
        return Some(installed_response(req, store).await);
    }
    // `/api/v1/skills/<id-or-name>[/usage|/telemetry]` — single-skill
    // drill-in. Must come after the exact-match routes above. Both
    // sub-routes accept either the catalog UUID (the SPA's skill-detail
    // page keys on it) OR the skill name (marketplace card / older SPA
    // paths), so a name-style segment resolves instead of 404-ing. The
    // bare detail path (`skill_detail_response`) still resolves by id
    // only.
    if let Some(rest) = bare.strip_prefix("/api/v1/skills/") {
        if let Some(id) = rest.strip_suffix("/usage") {
            if !id.is_empty() && !id.contains('/') {
                return Some(super::routes_skills_usage::skill_usage_response(req, store, id).await);
            }
        } else if let Some(id) = rest.strip_suffix("/telemetry") {
            if !id.is_empty() && !id.contains('/') {
                return Some(
                    super::routes_skills_telemetry::skill_telemetry_response(req, store, id).await,
                );
            }
        } else if !rest.is_empty() && !rest.contains('/') {
            return Some(skill_detail_response(req, store, rest).await);
        }
    }
    None
}

async fn installed_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant_id = match request_tenant_id(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.version, s.frontmatter, s.source, s.vertical_id, \
                s.created_at \
         FROM skill s \
         WHERE s.tenant_id = ? \
           AND s.superseded_by IS NULL \
           AND s.lifecycle_state = 'active' \
         ORDER BY s.name ASC",
    )
    .bind(&tenant_id)
    .fetch_all(store.pool())
    .await;
    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json!({"error": format!("list skills: {e}")}).to_string(),
            )
        }
    };
    let skills: Vec<Value> = rows.into_iter().map(skill_row_json).collect();
    let total_count = skills.len();
    let payload = json!({
        "skills": skills,
        "total_count": total_count,
        "tenant_id": tenant_id,
    });
    ("200 OK", payload.to_string())
}

/// Cross-tenant catalog: every published skill, regardless of which
/// tenant published it. Identifying tenant info is stripped from the
/// response. Used by the SPA's `/skills` marketplace page when the
/// caller wants to browse the un-installed catalog.
async fn marketplace_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.version, s.frontmatter, s.source, s.vertical_id, \
                s.created_at \
         FROM skill s \
         WHERE s.superseded_by IS NULL \
           AND s.lifecycle_state = 'active' \
           AND s.source LIKE 'published:%' \
         ORDER BY s.name ASC",
    )
    .fetch_all(store.pool())
    .await;
    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json!({"error": format!("list marketplace skills: {e}")}).to_string(),
            )
        }
    };
    let skills: Vec<Value> = rows.into_iter().map(skill_row_json).collect();
    let total_count = skills.len();
    let payload = json!({
        "skills": skills,
        "total_count": total_count,
    });
    ("200 OK", payload.to_string())
}

/// `GET /api/v1/skills/<id>` — single installed skill by UUID, scoped
/// to the caller's tenant. Returns the canonical skill envelope plus
/// the SKILL.md `body` + `created_at` the list omits, so the SPA's
/// skill-detail drill-in renders the full record. 404 if the id is
/// malformed or not an active skill in this tenant.
async fn skill_detail_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    id_str: &str,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant_id = match request_tenant_id(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let Ok(uuid) = Uuid::parse_str(id_str) else {
        return (
            "404 Not Found",
            json!({ "error": format!("invalid skill id '{id_str}'") }).to_string(),
        );
    };
    let id_bytes = uuid.as_bytes().to_vec();
    let row = sqlx::query(
        "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.vertical_id, \
                s.created_at \
         FROM skill s \
         WHERE s.id = ? \
           AND s.tenant_id = ? \
           AND s.superseded_by IS NULL \
           AND s.lifecycle_state = 'active' \
         LIMIT 1",
    )
    .bind(&id_bytes)
    .bind(&tenant_id)
    .fetch_optional(store.pool())
    .await;
    match row {
        Ok(Some(r)) => ("200 OK", skill_detail_json(r).to_string()),
        Ok(None) => (
            "404 Not Found",
            json!({ "error": format!("no active skill '{id_str}' in this tenant") }).to_string(),
        ),
        Err(e) => (
            "500 Internal Server Error",
            json!({ "error": format!("skill detail: {e}") }).to_string(),
        ),
    }
}

/// `GET /api/v1/skills/learned-this-month` — skill ids the caller's
/// tenant has learned since the first of the month (UTC). Mirrors the
/// bare `routes_skills` route's `{ since, count, skill_ids }` shape so
/// the marketplace "learned this month" chip can intersect it with the
/// installed list. Real data: a `worker_skill.learned_at` query.
async fn learned_this_month_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant_str = match request_tenant_id(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let Ok(uuid) = Uuid::parse_str(&tenant_str) else {
        return (
            "500 Internal Server Error",
            json!({ "error": format!("tenant id not a uuid: '{tenant_str}'") }).to_string(),
        );
    };
    let since = month_start_iso(chrono::Utc::now());
    let identity = IdentityStore::new(store);
    match identity.skill_ids_learned_since(TenantId(uuid), &since).await {
        Ok(ids) => (
            "200 OK",
            json!({ "since": since, "count": ids.len(), "skill_ids": ids }).to_string(),
        ),
        Err(e) => (
            "500 Internal Server Error",
            json!({ "error": format!("learned_since: {e}") }).to_string(),
        ),
    }
}

/// First-of-month (UTC) for `now` as an RFC3339 string.
fn month_start_iso(now: chrono::DateTime<chrono::Utc>) -> String {
    let d = now.date_naive();
    let first = chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1)
        .unwrap_or(d)
        .and_time(chrono::NaiveTime::MIN);
    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(first, chrono::Utc).to_rfc3339()
}

/// Full single-skill envelope: the catalog row shape (via
/// [`skill_row_json`]) extended with `body` + `created_at`.
fn skill_detail_json(row: sqlx::sqlite::SqliteRow) -> Value {
    let body: String = row.try_get("body").unwrap_or_default();
    let created_at: String = row.try_get("created_at").unwrap_or_default();
    let mut base = skill_row_json(row);
    if let Value::Object(ref mut map) = base {
        map.insert("body".to_string(), json!(body));
        map.insert("created_at".to_string(), json!(created_at));
    }
    base
}

/// Decode the response row into the canonical envelope shape. The
/// frontmatter is parsed best-effort so a malformed row still
/// surfaces its id/name pair instead of 500-ing the whole list.
fn skill_row_json(row: sqlx::sqlite::SqliteRow) -> Value {
    let id_blob: Vec<u8> = row.try_get("id").unwrap_or_default();
    let id = if id_blob.len() == 16 {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&id_blob);
        uuid::Uuid::from_bytes(arr).to_string()
    } else {
        String::new()
    };
    let name: String = row.try_get("name").unwrap_or_default();
    let version: String = row.try_get("version").unwrap_or_default();
    let source: String = row.try_get("source").unwrap_or_default();
    let vertical_id: Option<String> = row.try_get("vertical_id").ok();
    let frontmatter_str: String = row.try_get("frontmatter").unwrap_or_default();
    let (description, risk, tags) = parse_frontmatter_facets(&frontmatter_str);
    json!({
        "id": id,
        "name": name,
        "version": version,
        "description": description,
        "layer": "workspace",
        "risk": risk,
        "tags": tags,
        "vertical_id": vertical_id,
        "provenance_source": source,
        "trust_grade": TrustGrade::from_source(&source),
    })
}

/// Pull description/risk/tags out of the JSON-encoded frontmatter
/// blob. Returns sensible defaults on parse failure so the catalog
/// row still renders.
fn parse_frontmatter_facets(raw: &str) -> (String, String, Vec<String>) {
    let parsed: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let description = parsed
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let risk = parsed
        .get("risk")
        .and_then(Value::as_str)
        .unwrap_or("medium")
        .to_string();
    let tags = parsed
        .get("tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    (description, risk, tags)
}

/// Resolve the caller's tenant id. Prefers the `X-Argo-Tenant-Id`
/// header (set by the SPA proxy after JWT verification); falls back
/// to the cwd-bound workspace tenant for CLI / fixture callers.
pub(super) async fn request_tenant_id(
    req: &str,
    store: &SessionStore,
) -> Result<String, (&'static str, String)> {
    if let Some(id) = header_value(req, "X-Argo-Tenant-Id").filter(|s| !s.is_empty()) {
        return Ok(id);
    }
    let cwd = std::env::current_dir().map_err(|e| {
        (
            "500 Internal Server Error",
            json!({"error": format!("cwd: {e}")}).to_string(),
        )
    })?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    store
        .ensure_workspace(name, &root)
        .await
        .map(|(tenant, _)| tenant.to_string())
        .map_err(|e| {
            (
                "500 Internal Server Error",
                json!({"error": format!("resolve tenant: {e}")}).to_string(),
            )
        })
}

fn header_value(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}

#[cfg(test)]
#[path = "routes_skills_list_get_tests.rs"]
mod tests;
