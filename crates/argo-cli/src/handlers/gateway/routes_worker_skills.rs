//! `/api/v1/workers/<id>/skills{,/<sid>{,/history,/promote}}` — wave-3 API.
//!
//! Five endpoints behind the locked C2 worker-detail Skills tab:
//!
//! 1. `GET    /api/v1/workers/<id>/skills`               → Active / Library / Learned bundle.
//! 2. `POST   /api/v1/workers/<id>/skills`               → add a library skill to active.
//! 3. `DELETE /api/v1/workers/<id>/skills/<sid>`         → remove an active skill.
//! 4. `GET    /api/v1/workers/<id>/skills/<sid>/history` → audit-row timeline for one skill.
//! 5. `POST   /api/v1/workers/<id>/skills/<sid>/promote` → trust-ratchet promotion + audit.
//!
//! Sensible deviations from the brief:
//! - **JSON shape is camelCase** (`trustScore`, `usageCount7d`) to
//!   match the existing `web/lib/api/worker-skills.ts` interfaces.
//!   Gateway convention elsewhere is snake_case, but adding a parallel
//!   transform layer in the web client to flip naming on every render
//!   is more churn than this surface deserves. The wrapper key
//!   `total_counts` stays snake_case per the brief.
//! - **`learned` is empty in v1.** No learned-pattern table exists
//!   yet (separate brief). Promote still does real work — it copies
//!   the skill into the `worker_skill` overlay and writes a
//!   `SkillCandidatePromoted` audit row, so the trust ratchet leaves a
//!   visible trail on `/audit` today.
//! - **`usage_count_7d` placeholder is `0`.** Computing it requires
//!   a payload-LIKE walk of `audit_record` that bloats v1 reads; ships
//!   in `mission-worker-skills-usage-counters`.

use super::routes_worker_skills_helpers as helpers;
use argo_audit::AuditKind;
use argo_identity::{IdentityStore, WorkerId};
use argo_skill::{trust::TrustGrade, SkillStore};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// `GET /api/v1/workers/<id>/skills` — three-section bundle for the
/// locked C2 worker-detail Skills tab. `active` ∩ `library` = ∅ (an
/// active skill is filtered out of the library list).
pub(super) async fn list_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match helpers::parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ctx = match helpers::resolve_worker_ctx(store, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let skills = SkillStore::new(Arc::clone(store));
    let identity = IdentityStore::new(store);
    let reader = crate::handlers::worker_trust_reader::IdentityWorkerTrustReader::new(&identity);

    // `worker_skill.skill_id` is TEXT (UUID-hyphenated) but `skill.id`
    // is stored as BLOB — the canonical `SkillStore::list_for_worker`
    // JOIN is broken until that schema mismatch is fixed in a separate
    // brief. Compute the active set in two reads instead so this
    // endpoint surfaces real data today.
    let proficiency_map: HashMap<String, i64> = match identity.skills_for_worker(worker_id).await {
        Ok(v) => v.into_iter().collect(),
        Err(e) => return helpers::server_error(&format!("proficiency lookup: {e}")),
    };
    let active_ids: HashSet<String> = match active_skill_ids(store, worker_id).await {
        Ok(s) => s,
        Err(e) => return helpers::server_error(&format!("active ids: {e}")),
    };
    let available = match skills
        .available_for_worker(&reader, ctx.tenant, worker_id.0)
        .await
    {
        Ok(v) => v,
        Err(e) => return helpers::server_error(&format!("available: {e}")),
    };

    let mut active_json: Vec<Value> = Vec::with_capacity(active_ids.len());
    let mut library_json: Vec<Value> = Vec::with_capacity(available.len());
    for skill in available {
        let id_str = skill.id.0.to_string();
        let min = skills
            .get_min_trust_grade(&skill.frontmatter.name)
            .await
            .ok()
            .flatten()
            .map(trust_grade_ord);
        if active_ids.contains(&id_str) {
            let prof = proficiency_map
                .get(&id_str)
                .copied()
                .unwrap_or(helpers::DEFAULT_PROFICIENCY);
            active_json.push(helpers::active_skill_json(&skill, prof, min));
        } else {
            library_json.push(helpers::library_skill_json(&skill, min));
        }
    }

    let body = json!({
        "active": active_json,
        "library": library_json,
        "learned": Vec::<Value>::new(),
        "total_counts": {
            "active": active_json.len(),
            "library": library_json.len(),
            "learned": 0,
        },
    });
    helpers::emit_audit(
        store,
        req,
        &ctx,
        AuditKind::HttpRequest,
        "worker.skills.viewed",
        json!({
            "active_count": active_json.len(),
            "library_count": library_json.len(),
        }),
    )
    .await;
    ("200 OK", body.to_string())
}

/// `POST /api/v1/workers/<id>/skills` — add a library skill to the
/// worker's active overlay (`worker_skill` row). Idempotent: a repeat
/// call upserts the proficiency without erroring.
pub(super) async fn add_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match helpers::parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let (skill_id, trust_grade_min) = match helpers::parse_post_body(req) {
        Ok(t) => t,
        Err(r) => return r,
    };
    let ctx = match helpers::resolve_worker_ctx(store, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let identity = IdentityStore::new(store);
    if let Err(e) = identity
        .add_worker_skill(worker_id, &skill_id.0.to_string(), helpers::DEFAULT_PROFICIENCY)
        .await
    {
        return helpers::server_error(&format!("add worker_skill: {e}"));
    }
    helpers::emit_audit(
        store,
        req,
        &ctx,
        AuditKind::HttpRequest,
        "worker.skills.added",
        json!({
            "skill_id": skill_id.0.to_string(),
            "trust_grade_min": trust_grade_min,
        }),
    )
    .await;
    (
        "201 Created",
        json!({
            "worker_id": worker_id.to_string(),
            "skill_id": skill_id.0.to_string(),
            "proficiency": helpers::DEFAULT_PROFICIENCY,
        })
        .to_string(),
    )
}

/// `DELETE /api/v1/workers/<id>/skills/<sid>` — drop the overlay row.
/// 204 even when the row is absent (idempotent), matching DELETE
/// semantics elsewhere in the gateway.
pub(super) async fn remove_response(
    worker_id_raw: &str,
    skill_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match helpers::parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let skill_id = match helpers::parse_skill_id(skill_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ctx = match helpers::resolve_worker_ctx(store, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let identity = IdentityStore::new(store);
    if let Err(e) = identity
        .remove_worker_skill(worker_id, &skill_id.0.to_string())
        .await
    {
        return helpers::server_error(&format!("remove worker_skill: {e}"));
    }
    helpers::emit_audit(
        store,
        req,
        &ctx,
        AuditKind::HttpRequest,
        "worker.skills.removed",
        json!({ "skill_id": skill_id.0.to_string() }),
    )
    .await;
    ("204 No Content", String::new())
}

/// `GET /api/v1/workers/<id>/skills/<sid>/history` — tenant-scoped
/// audit-row timeline for one skill. Useful for "when did this
/// worker learn this skill?" and trust-ratchet provenance.
pub(super) async fn history_response(
    worker_id_raw: &str,
    skill_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match helpers::parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let skill_id = match helpers::parse_skill_id(skill_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ctx = match helpers::resolve_worker_ctx(store, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let events = match helpers::fetch_history(store, ctx.tenant, worker_id, skill_id).await {
        Ok(v) => v,
        Err(e) => return helpers::server_error(&format!("history lookup: {e}")),
    };
    helpers::emit_audit(
        store,
        req,
        &ctx,
        AuditKind::HttpRequest,
        "worker.skills.history.viewed",
        json!({
            "skill_id": skill_id.0.to_string(),
            "event_count": events.len(),
        }),
    )
    .await;
    (
        "200 OK",
        json!({
            "worker_id": worker_id.to_string(),
            "skill_id": skill_id.0.to_string(),
            "events": events,
        })
        .to_string(),
    )
}

/// `POST /api/v1/workers/<id>/skills/<sid>/promote` — trust-ratchet
/// promotion. v1 implementation: upsert the `worker_skill` overlay row
/// at the highest proficiency (5) and write a `SkillCandidatePromoted`
/// audit row so the ratchet is visible on `/audit`. When a real
/// learned-pattern table lands the body can swap the source side
/// without changing the gateway surface.
pub(super) async fn promote_response(
    worker_id_raw: &str,
    skill_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match helpers::parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let skill_id = match helpers::parse_skill_id(skill_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ctx = match helpers::resolve_worker_ctx(store, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let identity = IdentityStore::new(store);
    // Promotion upserts at proficiency 5 — "fully ratcheted." Add a
    // separate column for the learned→active state when the learned
    // table arrives.
    if let Err(e) = identity
        .add_worker_skill(worker_id, &skill_id.0.to_string(), 5)
        .await
    {
        return helpers::server_error(&format!("promote: {e}"));
    }
    helpers::emit_audit(
        store,
        req,
        &ctx,
        AuditKind::SkillCandidatePromoted,
        "worker.skills.promoted",
        json!({
            "skill_id": skill_id.0.to_string(),
            "promotion_kind": "learned_to_active",
        }),
    )
    .await;
    (
        "200 OK",
        json!({
            "worker_id": worker_id.to_string(),
            "skill_id": skill_id.0.to_string(),
            "promoted": true,
        })
        .to_string(),
    )
}

/// Dispatch entry point for `/api/v1/workers/<id>/skills*` write
/// verbs. Returns `Some(resp)` on a match so the caller falls through
/// when no arm matches. Mounted from `routes_dispatch_post.rs`.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(rest) = req.strip_prefix("POST /api/v1/workers/") {
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        // [worker_id, "skills"] → add to overlay
        if parts.len() == 2 && parts[1] == "skills" {
            return Some(add_response(parts[0], req, store).await);
        }
        // [worker_id, "skills", skill_id, "promote"] → trust ratchet
        if parts.len() == 4 && parts[1] == "skills" && parts[3] == "promote" {
            return Some(promote_response(parts[0], parts[2], req, store).await);
        }
    }
    if let Some(rest) = req.strip_prefix("DELETE /api/v1/workers/") {
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        // [worker_id, "skills", skill_id] → remove overlay
        if parts.len() == 3 && parts[1] == "skills" {
            return Some(remove_response(parts[0], parts[2], req, store).await);
        }
    }
    None
}

/// Project a `TrustGrade` to its ordinal byte for the trust-score
/// heuristic. Ordering matches `argo_skill::trust::TrustGrade`'s
/// most-to-least-trusted enum order (`Builtin` = 4, ...,
/// `LearnedUnreviewed` = 0).
fn trust_grade_ord(grade: TrustGrade) -> u8 {
    match grade {
        TrustGrade::Builtin => 4,
        TrustGrade::PublishedVerified => 3,
        TrustGrade::Tenant => 2,
        TrustGrade::PublishedCommunity => 1,
        TrustGrade::LearnedUnreviewed => 0,
    }
}

/// Direct read of the `worker_skill.skill_id` column for `worker_id`.
/// Bypasses `SkillStore::list_for_worker` while its `JOIN skill` is
/// broken by the BLOB/TEXT schema mismatch on `skill.id`. Returns the
/// TEXT-form UUIDs the table stores so callers can compare against
/// `Skill::id.0.to_string()`.
async fn active_skill_ids(
    store: &Arc<SessionStore>,
    worker_id: WorkerId,
) -> ArgoResult<HashSet<String>> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT skill_id FROM worker_skill WHERE worker_id = ?")
            .bind(worker_id.to_string())
            .fetch_all(store.pool())
            .await
            .map_err(|e| argo_types::ArgoError::Provider(format!("active_skill_ids: {e}")))?;
    Ok(rows.into_iter().collect())
}

#[cfg(test)]
#[path = "routes_worker_skills_tests.rs"]
mod tests;
