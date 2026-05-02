//! `POST /sessions/<id>/clone` and `POST /sessions/<id>/branch
//! ?through-seq=N` — duplicate a session into a fresh session
//! row. The clone/branch gets a new id, a "(clone)" or
//! "(branch @<seq>)" suffix on its title, and a fresh audit
//! chain (no audit rows are copied — clone/branch are a fork,
//! not a re-run).
//!
//! - `clone` copies every message verbatim. Useful for
//!   "give me a sandbox to mess with this session".
//! - `branch?through-seq=N` copies messages 1..=N only. Useful
//!   for "rewind to turn 5 and try a different prompt"
//!   experimentation: branch, then post a new prompt.
use super::store_unavailable;
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId};
use std::sync::Arc;

#[derive(Copy, Clone)]
enum Mode {
    Clone,
    Branch,
}

pub(super) async fn session_clone_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    clone_or_branch_response(sid, store, None, Mode::Clone).await
}

pub(super) async fn session_branch_response(
    sid: &str,
    through_seq: Option<i64>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    clone_or_branch_response(sid, store, through_seq, Mode::Branch).await
}

async fn clone_or_branch_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    through_seq: Option<i64>,
    mode: Mode,
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
    let suffix = match (mode, through_seq) {
        (Mode::Branch, Some(n)) => format!("(branch @{n})"),
        (Mode::Branch, None) => String::from("(branch)"),
        (Mode::Clone, _) => String::from("(clone)"),
    };
    let new_title = match header.title.as_deref() {
        Some(t) if !t.is_empty() => format!("{t} {suffix}"),
        _ => suffix.clone(),
    };
    let new_id = match store
        .create_session(
            tenant,
            workspace,
            &header.model,
            &header.provider,
            Some(new_title.as_str()),
            header.vertical_id.as_deref(),
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
    let mut rows = match store.list_messages(id).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"list_messages: {e}\"}}"),
            );
        }
    };
    if let Some(n) = through_seq {
        rows.retain(|r| r.seq <= n);
    }
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
    let payload = match mode {
        Mode::Branch => serde_json::json!({
            "branched_from": sid,
            "session_id": new_id.0.to_string(),
            "title": new_title,
            "through_seq": through_seq,
            "messages_copied": copied,
        }),
        Mode::Clone => serde_json::json!({
            "cloned_from": sid,
            "session_id": new_id.0.to_string(),
            "title": new_title,
            "messages_copied": copied,
        }),
    };
    (
        "201 Created",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}
