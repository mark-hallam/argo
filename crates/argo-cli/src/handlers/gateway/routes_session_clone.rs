//! `POST /sessions/<id>/clone` — duplicate a session, copying
//! every message into a fresh session row. The clone gets a
//! new id + a "(clone)" suffix on its title; the new audit
//! chain starts fresh (no audit rows are copied — clones are
//! a fork, not a re-run). Useful for "what if I'd said
//! something different at turn 5" experimentation: clone, then
//! delete the tail with /sessions/<new_id>/messages/... API
//! work, then post a new prompt.
use super::store_unavailable;
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId};
use std::sync::Arc;

pub(super) async fn session_clone_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let header = match store.get_session(id).await {
        Ok(Some(h)) => h,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no session {sid}\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"get_session: {e}\"}}"),
            );
        }
    };
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = match store.ensure_workspace(name, &root).await {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"ensure_workspace: {e}\"}}"),
            );
        }
    };
    let new_title = match header.title.as_deref() {
        Some(t) if !t.is_empty() => format!("{t} (clone)"),
        _ => String::from("(clone)"),
    };
    let new_id = match store
        .create_session(
            tenant,
            workspace,
            &header.model,
            &header.provider,
            Some(new_title.as_str()),
        )
        .await
    {
        Ok(i) => i,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"create_session: {e}\"}}"),
            );
        }
    };
    let rows = match store.list_messages(id).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"list_messages: {e}\"}}"),
            );
        }
    };
    let mut copied: u64 = 0;
    for r in rows {
        let role = match r.role.as_str() {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            other => {
                return (
                    "500 Internal Server Error",
                    format!(
                        "{{\"error\":\"unknown role '{other}' in source session\"}}"
                    ),
                );
            }
        };
        if let Err(e) = store
            .append_message(new_id, role, &r.content_json)
            .await
        {
            return (
                "500 Internal Server Error",
                format!(
                    "{{\"error\":\"append_message #{copied}: {e}\"}}"
                ),
            );
        }
        copied += 1;
    }
    (
        "201 Created",
        serde_json::to_string(&serde_json::json!({
            "cloned_from": sid,
            "session_id": new_id.0.to_string(),
            "title": new_title,
            "messages_copied": copied,
        }))
        .unwrap_or_else(|_| "{}".to_string()),
    )
}
