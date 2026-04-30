//! `argo steer <session_id> <text>` — inject a steer into a
//! recorded session.
//!
//! Appends a `system` message containing the steer text, so the
//! next `argo chat` (or daemon-driven turn) sees it as part of
//! the prompt prefix. The §A14 spec calls out that mid-run
//! injection happens via the daemon's `SteerChannel`; outside a
//! live run, this offline shim is the equivalent — the next
//! provider call will treat it as if the user steered.
//!
//! The system message is wrapped in a `text` content block tagged
//! with `kind = "steer"` in `provider_meta` (TODO once we extend
//! `append_message` to carry meta) so future tooling can filter
//! steers from regular system prompts.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, MessageRole};

pub async fn run(raw: &str, text: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let session_id = super::session_resolve::resolve(&store, raw).await?;
    let id = session_id.0;
    // Confirm session exists so we don't append into thin air.
    if store.get_session(session_id).await?.is_none() {
        return Err(ArgoError::Provider(format!("no session {raw}")));
    }
    let body = format!("[steer] {text}");
    let content = serde_json::json!([{ "type": "text", "text": body }]);
    let msg_id = store
        .append_message(session_id, MessageRole::System, &content.to_string())
        .await?;
    println!("argo steer: appended system message {msg_id} to session {id}");
    println!("  next `argo chat {id}` will see this as a system-prefix steer.");
    Ok(())
}
