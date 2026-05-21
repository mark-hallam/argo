//! Internal helpers for [`super::routes_worker_skills`]. Split out so
//! both files stay under the 500-LOC HARD RULE 6 ceiling. Items are
//! `pub(super)`-only and not part of the gateway's public surface.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_skill::Skill;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// Active proficiency assigned by POST when the body omits one.
/// Mid-ladder (1..5) leaves head-room for explicit grades either way.
pub(super) const DEFAULT_PROFICIENCY: i64 = 3;

/// Resolved worker plus the workspace tenant the gateway is serving.
pub(super) struct WorkerCtx {
    pub(super) tenant: TenantId,
    pub(super) worker: Worker,
}

pub(super) fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    t.parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{t}': {e}")))
}

pub(super) fn parse_skill_id(raw: &str) -> Result<SkillId, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    Uuid::parse_str(t)
        .map(SkillId)
        .map_err(|e| bad_request(&format!("invalid skill_id '{t}': {e}")))
}

/// Resolve the workspace-bound tenant + look the worker up under it.
/// Cross-tenant / unknown both surface as 404 — never leak existence.
pub(super) async fn resolve_worker_ctx(
    store: &Arc<SessionStore>,
    worker_id: WorkerId,
) -> Result<WorkerCtx, (&'static str, String)> {
    let tenant = resolve_workspace_tenant(store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    match IdentityStore::new(store).get_worker(tenant, worker_id).await {
        Ok(Some(worker)) => Ok(WorkerCtx { tenant, worker }),
        Ok(None) => Err(not_found(&format!("worker {worker_id} not found"))),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

/// Read the JSON `{ "skill_id": "<uuid>", ... }` body and pull the id.
/// `trust_grade_min` is accepted but optional — v1 stores no per-overlay
/// floor (skills already carry one); the field round-trips into audit.
pub(super) fn parse_post_body(req: &str) -> Result<(SkillId, Option<i64>), (&'static str, String)> {
    let raw = super::body_from_request(req);
    let parsed: Value = serde_json::from_str(raw)
        .map_err(|e| bad_request(&format!("parse JSON body: {e}")))?;
    let id_str = parsed
        .get("skill_id")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("missing `skill_id` field"))?;
    let skill_id = Uuid::parse_str(id_str)
        .map(SkillId)
        .map_err(|e| bad_request(&format!("invalid skill_id '{id_str}': {e}")))?;
    let trust_grade_min = parsed.get("trust_grade_min").and_then(Value::as_i64);
    Ok((skill_id, trust_grade_min))
}

/// Map `skill.frontmatter.provenance.source` to the three-value enum
/// the locked Skills tab uses (`workspace` / `shared` / `marketplace`).
pub(super) fn map_source(raw: &str) -> &'static str {
    match raw {
        "published" | "published_verified" | "published_community" | "marketplace"
        | "builtin" | "verticals" => "marketplace",
        "shared" | "cluster" | "team" => "shared",
        // Workspace / tenant / learned / anything else
        _ => "workspace",
    }
}

/// Heuristic trust score 0..100 derived from the skill's
/// `min_trust_grade` floor. NULL → mid (no gate). The Skills UI's
/// `TrustBadge` buckets at ≥90 (success) and ≥70 (warning).
pub(super) fn trust_score(min_trust_grade: Option<u8>) -> u32 {
    match min_trust_grade {
        None => 82,
        Some(0) => 60,
        Some(1) => 70,
        Some(2) => 80,
        Some(3) => 90,
        Some(4) => 95,
        Some(_) => 80,
    }
}

/// Project an active overlay row into the camelCase shape consumed by
/// `web/lib/api/worker-skills.ts::ActiveSkill`. `proficiency` is the
/// 1..5 ladder stored alongside `worker_skill`; `usage_count_7d` is a
/// v1 zero placeholder (the trailing audit-payload-LIKE query is a
/// follow-up brief). `learned_at` is the `worker_skill.created_at`
/// timestamp the learning-loop chip ("Learned 2 weeks ago") renders
/// against — `None` when the worker pre-dates the v1 capture.
pub(super) fn active_skill_json(
    skill: &Skill,
    proficiency: i64,
    min_trust: Option<u8>,
    learned_at: Option<&str>,
) -> Value {
    json!({
        "id": skill.id.0.to_string(),
        "name": skill.frontmatter.name,
        "version": skill.frontmatter.version,
        "trustScore": trust_score(min_trust),
        "usageCount7d": 0,
        "proficiency": proficiency,
        "learnedAt": learned_at,
    })
}

/// Project a library row (skill in the available set, not currently
/// active for the worker) into `LibrarySkill` shape.
pub(super) fn library_skill_json(skill: &Skill, min_trust: Option<u8>) -> Value {
    json!({
        "id": skill.id.0.to_string(),
        "name": skill.frontmatter.name,
        "version": skill.frontmatter.version,
        "trustScore": trust_score(min_trust),
        "source": map_source(&skill.frontmatter.provenance.source),
    })
}

/// Fire-and-log audit row for the gateway-facing skill endpoints.
/// `HttpRequest+endpoint=...` mirrors `routes_worker_memory` so the
/// audit feed groups all worker-detail-page reads/writes together.
/// Promote uses `SkillCandidatePromoted` instead so the trust ratchet
/// stands out on the `/audit` timeline (acceptance criterion).
pub(super) async fn emit_audit(
    store: &Arc<SessionStore>,
    req: &str,
    ctx: &WorkerCtx,
    kind: AuditKind,
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
        .write(None, kind, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(target: "gateway::worker_skills", error = %e, "audit write failed");
    }
}

/// Tenant-scoped audit-row history for `(worker_id, skill_id)`. Reads
/// the raw `audit_record.payload_json` and matches rows whose payload
/// references the skill — covers both the gateway-emitted rows from
/// this brief and any organic `skill.*` rows already in the chain.
pub(super) async fn fetch_history(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
    skill_id: SkillId,
) -> ArgoResult<Vec<Value>> {
    // Cap defensive — the worker-detail page only renders the most
    // recent ~50 events; full timeline lives in `/audit?worker_id=...`.
    let needle_skill = skill_id.0.to_string();
    let needle_worker = worker_id.to_string();
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT kind, payload_json, created_at FROM audit_record \
         WHERE tenant_id = ? \
           AND (payload_json LIKE ? OR payload_json LIKE ?) \
           AND payload_json LIKE ? \
         ORDER BY created_at DESC LIMIT 50",
    )
    .bind(tenant.to_string())
    .bind(format!("%{needle_skill}%"))
    .bind(format!("%{}%", skill_id.0.as_simple()))
    .bind(format!("%{needle_worker}%"))
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("history lookup: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(kind, payload, created_at)| {
            let payload_v: Value =
                serde_json::from_str(&payload).unwrap_or(Value::Null);
            json!({
                "kind": kind,
                "at": created_at,
                "payload": payload_v,
            })
        })
        .collect())
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
