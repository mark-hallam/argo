//! Skill-write handlers extracted from `routes_post_writes.rs`:
//! `POST /skills` (install) and `POST /skills/<name>/promote`.
//! Lives in its own module so `routes_post_writes.rs` (the rest
//! of the write-side POSTs) stays well under the 500-LOC ceiling.
//! Both operate on the workspace `SkillStore` directly.
use super::{body_from_request, store_unavailable};
use argo_store::SessionStore;
use std::sync::Arc;

/// `POST /skills` — install a new skill from a raw SKILL.md
/// document in the request body. Returns 201 with the new
/// skill id on success; 400 on parse / duplicate / lock errors.
pub(super) async fn install_skill_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse JSON body: {e}\"}}"),
            );
        }
    };
    let raw = match parsed.get("raw").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return (
                "400 Bad Request",
                "{\"error\":\"missing or empty `raw` (full SKILL.md document)\"}".to_string(),
            );
        }
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.install(&raw, None).await {
        Ok(s) => {
            let body = serde_json::json!({
                "id": s.id.0.to_string(),
                "name": s.frontmatter.name,
                "version": s.frontmatter.version,
            });
            (
                "201 Created",
                serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"install: {e}\"}}"),
        ),
    }
}

/// `POST /skills/<name>/promote` — promote a specific skill
/// version to active. Mirrors `argo forge promote --skill X
/// --version Y`. Body shape: `{"version":"<x.y>"}`. Returns
/// 200 + `{name, version, skill_id}` on success; 400 on
/// missing version / unknown skill; 503 on store-unavailable.
pub(super) async fn skill_promote_response(
    name: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse JSON body: {e}\"}}"),
            );
        }
    };
    let Some(version) = parsed
        .get("version")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
    else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing or empty `version`\"}".to_string(),
        );
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.promote(name, version).await {
        Ok(s) => {
            let resp = serde_json::json!({
                "name": s.frontmatter.name,
                "version": s.frontmatter.version,
                "skill_id": s.id.0.to_string(),
            });
            (
                "200 OK",
                serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"promote: {e}\"}}"),
        ),
    }
}
