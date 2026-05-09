//! Write-side POST handlers for non-session resources —
//! `POST /skills`, `POST /skills/<name>/promote`, `POST /memory`,
//! `POST /forge/{optimize,bench}`, `POST /forge/runs/<id>/abort`.
//! Lives in its own module so `routes_post.rs` (the session-
//! flavoured POSTs) stays under the 500-LOC ceiling.
use super::{body_from_request, store_unavailable};
use crate::home::cwd_workspace;
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
    // #718: cwd-bound workspace (matches every reader's path).
    let workspace = match cwd_workspace(store).await {
        Ok(w) => w,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd_workspace: {e}\"}}"),
            );
        }
    };
    let memory = argo_memory::MemoryStore::new(Arc::clone(store));
    // #687 — vertical-aware append. When the body carries a
    // `vertical_id` string, route through `append_for_vertical`
    // so the new row is tagged for the per-vertical lookup
    // (#680, Rule 7). Without it, the legacy workspace-global
    // path is preserved.
    let vertical = parsed
        .get("vertical_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let result = match &vertical {
        Some(vid) => {
            memory
                .append_for_vertical(workspace, file, body_text, None, vid)
                .await
        }
        None => memory.append(workspace, file, body_text, None).await,
    };
    match result {
        Ok(entry) => {
            let resp = serde_json::json!({
                "id": entry.id.to_string(),
                "file": canonical,
                "vertical_id": entry.vertical_id,
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

// Skill-write handlers (`install_skill_response`,
// `skill_promote_response`) live in `routes_post_writes_skills`.
pub(super) use super::routes_post_writes_skills::{
    install_skill_response, skill_promote_response,
};

// `forge_optimize_response` lives in the sibling
// `routes_post_forge` module since v1.1 (#581) — the
// new enqueue-by-default path needs a SessionStore handle
// plus a skill-store lookup, which made it large enough to
// push this file over the 500-LOC ceiling.
pub(super) use super::routes_post_forge::forge_optimize_response;

/// `POST /forge/bench` — kick off `argo forge bench` in the
/// background. Body shape:
/// `{"skill": "...", "examples": "<path>", "provider": "...",
///   "model": "..."}` (provider + model optional, default to
/// anthropic / claude-3-5-sonnet-20241022). Returns 202
/// Accepted; caller polls `GET /forge/runs` for the result.
pub(super) async fn forge_bench_response(req: &str) -> (&'static str, String) {
    let parsed = match parse_forge_body(req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let skill = parsed.skill;
    let examples = parsed.examples;
    let provider = parsed
        .raw
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("anthropic")
        .to_string();
    let model = parsed
        .raw
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("claude-3-5-sonnet-20241022")
        .to_string();
    crate::handlers::forge::spawn_bench(
        skill.clone(),
        examples.clone(),
        provider.clone(),
        model.clone(),
    );
    let resp = serde_json::json!({
        "started": true,
        "kind": "bench",
        "skill": skill,
        "examples": examples,
        "provider": provider,
        "model": model,
    });
    (
        "202 Accepted",
        serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `POST /forge/runs/<id>/abort` — flip a forge_run row's
/// status to `"aborted"` and stamp `ended_at`. Mirrors
/// `argo forge runs abort <id>`. The path id may be a full
/// UUID; short prefixes are rejected with 400 (matching the
/// CLI). Returns 200 + `{aborted:true,run_id}`, 404 when the
/// row doesn't exist, 400 on a malformed id.
pub(super) async fn forge_abort_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"invalid forge run id '{raw_id}' \
                     (pass the full uuid)\"}}"
                ),
            );
        }
    };
    match store.abort_forge_run(id).await {
        Ok(0) => (
            "404 Not Found",
            format!("{{\"error\":\"no forge run with id {raw_id}\"}}"),
        ),
        Ok(_) => (
            "200 OK",
            format!("{{\"aborted\":true,\"run_id\":\"{raw_id}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"abort_forge_run: {e}\"}}"),
        ),
    }
}

// SessionStore-admin handlers (`db_{vacuum,prune}_response`,
// `workspace_init_response`) live in `routes_post_writes_db`.
pub(super) use super::routes_post_writes_db::{
    db_prune_response, db_vacuum_response, workspace_init_response,
};

pub(super) struct ForgeBody {
    pub(super) skill: String,
    pub(super) examples: String,
    pub(super) raw: serde_json::Value,
}

pub(super) fn parse_forge_body(req: &str) -> Result<ForgeBody, (&'static str, String)> {
    let body = body_from_request(req);
    let parsed: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| ("400 Bad Request", format!("{{\"error\":\"parse JSON body: {e}\"}}")))?;
    let skill = parsed
        .get("skill")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .ok_or((
            "400 Bad Request",
            "{\"error\":\"missing or empty `skill`\"}".to_string(),
        ))?;
    let examples = parsed
        .get("examples")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .ok_or((
            "400 Bad Request",
            "{\"error\":\"missing or empty `examples` (path to JSONL)\"}".to_string(),
        ))?;
    Ok(ForgeBody {
        skill,
        examples,
        raw: parsed,
    })
}
