//! `POST /skills/<name>/refine` — append a `## Refinement`
//! block to the active skill's body and supersede it with a
//! bumped version. Mirrors `argo skill refine` (#271). Lives
//! in its own file so `routes_post_admin.rs` stays under the
//! 500-LOC budget.
use super::role_gate::{enforce_role, Action};
use super::routes_auth_principal::principal_id_for_audit;
use super::{body_from_request, request_tenant_id, store_unavailable};
use argo_audit::{AuditKind, AuditSink};
use argo_skill::{PrincipalOverride as SkillPrincipal, SkillStore, PLACEHOLDER_TENANT_ID};
use argo_store::SessionStore;
use argo_types::SkillId;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Body `{hint, version?}`. Returns 200 + `{refined, name,
/// prev_version, new_version, skill_id}`. 400 on missing hint
/// or supersede failure; 404 when the named skill has no
/// active row.
pub(super) async fn skill_refine_response(
    name: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::KnowledgeWrite) {
        return deny;
    }
    let Some(store) = store else {
        return store_unavailable();
    };
    let parsed: Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse JSON body: {e}\"}}"),
            );
        }
    };
    let Some(hint) = parsed
        .get("hint")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing or empty `hint`\"}".to_string(),
        );
    };
    let version_override = parsed
        .get("version")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let skills = SkillStore::new(Arc::clone(store));
    let active = match skills.find_active_by_name(name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no active skill named '{name}'\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"find_active_by_name: {e}\"}}"),
            );
        }
    };
    let tenant_id = request_tenant_id(req);
    let skill_tenant = match skill_tenant_id(store, active.id).await {
        Ok(t) => t,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"skill tenant lookup: {e}\"}}"),
            );
        }
    };
    if is_cross_tenant_request(&tenant_id, skill_tenant.as_deref()) {
        return no_active_skill(name);
    }
    let new_version =
        version_override.unwrap_or_else(|| bump_skill_version(&active.frontmatter.version));
    let new_body = format!("{}\n\n## Refinement\n{}\n", active.body.trim_end(), hint);
    let raw = format!(
        "---\nname: {name}\nversion: \"{new_version}\"\ndescription: {desc}\n\
         lock: none\nprovenance:\n  source: refined-from-{prev}\n  hash: sha256-{hash}\n---\n{body}",
        desc = active.frontmatter.description,
        prev = active.frontmatter.version,
        hash = sha256_hex(&new_body),
        body = new_body,
    );
    match skills.supersede(&raw, None, SkillPrincipal::Force).await {
        Ok(new_skill) => {
            if let Some(row_tenant) = skill_tenant
                .as_deref()
                .filter(|t| !is_placeholder_tenant(t))
            {
                if let Err(e) = stamp_skill_tenant(store, new_skill.id, row_tenant).await {
                    return (
                        "500 Internal Server Error",
                        format!("{{\"error\":\"skill tenant stamp: {e}\"}}"),
                    );
                }
            }
            let audit = RefineAudit {
                tenant_id: &tenant_id,
                name,
                previous_skill_id: active.id,
                skill_id: new_skill.id,
                prev_version: &active.frontmatter.version,
                new_version: &new_version,
                vertical_id: active.vertical_id.as_deref(),
            };
            emit_refine_audit(store, req, audit).await;
            let payload = json!({
                "refined": true,
                "name": name,
                "prev_version": active.frontmatter.version,
                "new_version": new_version,
                "skill_id": new_skill.id.0.to_string(),
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"supersede: {e}\"}}"),
        ),
    }
}

fn no_active_skill(name: &str) -> (&'static str, String) {
    (
        "404 Not Found",
        format!("{{\"error\":\"no active skill named '{name}'\"}}"),
    )
}

async fn skill_tenant_id(
    store: &SessionStore,
    skill_id: SkillId,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT tenant_id FROM skill WHERE id = ? LIMIT 1")
        .bind(skill_id.0.as_bytes().as_slice())
        .fetch_optional(store.pool())
        .await
}

async fn stamp_skill_tenant(
    store: &SessionStore,
    skill_id: SkillId,
    tenant_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE skill SET tenant_id = ? WHERE id = ?")
        .bind(tenant_id)
        .bind(skill_id.0.as_bytes().as_slice())
        .execute(store.pool())
        .await?;
    Ok(())
}

fn is_cross_tenant_request(request_tenant: &str, skill_tenant: Option<&str>) -> bool {
    matches!(skill_tenant, Some(t) if !is_placeholder_tenant(t) && t != request_tenant)
}

fn is_placeholder_tenant(tenant_id: &str) -> bool {
    tenant_id == PLACEHOLDER_TENANT_ID
}

struct RefineAudit<'a> {
    tenant_id: &'a str,
    name: &'a str,
    previous_skill_id: SkillId,
    skill_id: SkillId,
    prev_version: &'a str,
    new_version: &'a str,
    vertical_id: Option<&'a str>,
}

async fn emit_refine_audit(store: &Arc<SessionStore>, req: &str, audit: RefineAudit<'_>) {
    let principal_id = principal_id_for_audit(req);
    let payload = json!({
        "endpoint": "skill.refined",
        "tenant_id": audit.tenant_id,
        "name": audit.name,
        "previous_skill_id": audit.previous_skill_id.to_string(),
        "skill_id": audit.skill_id.to_string(),
        "prev_version": audit.prev_version,
        "new_version": audit.new_version,
        "vertical_id": audit.vertical_id,
        "refined_by": principal_id.as_deref(),
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), audit.tenant_id.to_string());
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id)
        .await
    {
        tracing::warn!(target: "gateway::skill_refine", error = %e, "skill refine audit write failed");
        return;
    }
    if let Err(e) = sink.flush_pending().await {
        tracing::warn!(target: "gateway::skill_refine", error = %e, "skill refine audit flush failed");
    }
}

fn bump_skill_version(v: &str) -> String {
    let mut parts: Vec<String> = v.split('.').map(String::from).collect();
    if let Some(last) = parts.last_mut() {
        if let Ok(n) = last.parse::<u64>() {
            *last = (n + 1).to_string();
            return parts.join(".");
        }
    }
    format!("{v}-refined")
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

#[cfg(test)]
#[path = "routes_skill_refine_tests.rs"]
mod tests;
