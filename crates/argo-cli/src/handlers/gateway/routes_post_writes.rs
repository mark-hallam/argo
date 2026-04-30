//! Write-side POST handlers for non-session resources —
//! `POST /skills`, `POST /memory`. Lives in its own module so
//! `routes_post.rs` (the session-flavoured POSTs) stays under
//! the 500-LOC ceiling.
use super::{body_from_request, store_unavailable};
use argo_store::SessionStore;
use std::sync::Arc;

/// `POST /memory` — append a new memory entry. Body shape:
/// `{"file": "memory|user|soul|agents", "body": "<text>"}`.
/// Returns 201 with the new entry id; 400 on invalid file name
/// / empty body; 503 on store-unavailable.
pub(super) async fn append_memory_response(
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
    let Some(file_str) = parsed.get("file").and_then(|v| v.as_str()) else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing `file` (memory|user|soul|agents)\"}".to_string(),
        );
    };
    let canonical = match file_str {
        "memory" | "MEMORY.md" => "MEMORY.md",
        "user" | "USER.md" => "USER.md",
        "soul" | "SOUL.md" => "SOUL.md",
        "agents" | "AGENTS.md" => "AGENTS.md",
        other => {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown file '{other}' — use memory|user|soul|agents\"}}"
                ),
            );
        }
    };
    let Some(file) = argo_memory::MemoryFile::parse(canonical) else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"MemoryFile::parse failed for canonical name\"}".to_string(),
        );
    };
    let body_text = parsed
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if body_text.trim().is_empty() {
        return (
            "400 Bad Request",
            "{\"error\":\"empty body\"}".to_string(),
        );
    }
    let workspace = match store.first_workspace().await {
        Ok(Some(w)) => w,
        Ok(None) => {
            return (
                "400 Bad Request",
                "{\"error\":\"no workspace registered yet — run `argo workspace init`\"}"
                    .to_string(),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"first_workspace: {e}\"}}"),
            );
        }
    };
    let memory = argo_memory::MemoryStore::new(Arc::clone(store));
    match memory.append(workspace, file, body_text, None).await {
        Ok(entry) => {
            let resp = serde_json::json!({
                "id": entry.id.to_string(),
                "file": canonical,
            });
            (
                "201 Created",
                serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"append: {e}\"}}"),
        ),
    }
}

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
