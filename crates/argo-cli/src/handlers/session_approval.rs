//! `argo session approve` + `argo session revoke-approval`
//! handlers (#664). Lives in its own module so `session.rs`
//! stays under the 500-LOC budget.
//!
//! Both handlers map directly onto the
//! `SessionStore::add_session_grant` and
//! `revoke_session_grants_for_tool` methods (added in #664). The
//! grant store is the single source of truth — the agent
//! runner re-reads it at every TurnStart so a CLI-issued
//! approval is visible to the next turn without any IPC.
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;
use uuid::Uuid;

/// Open the local store the same way every other session
/// handler does — via `argo-cli::home`. Returns an `Arc` so
/// audit + grant writes share the connection pool.
async fn open_store() -> ArgoResult<Arc<SessionStore>> {
    use crate::home::{db_path, ensure_dir, resolve_home};
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let pool = SessionStore::open(db_path(&home).to_string_lossy().as_ref()).await?;
    pool.migrate().await?;
    Ok(Arc::new(pool))
}

fn parse_session_id(s: &str) -> ArgoResult<SessionId> {
    let u = Uuid::parse_str(s)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{s}': {e}")))?;
    Ok(SessionId(u))
}

/// `argo session approve <session-id> --tool <name> [--note "<text>"]`.
pub async fn approve(
    session_id_str: &str,
    tool: &str,
    note: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let sid = parse_session_id(session_id_str)?;
    let store = open_store().await?;
    // Sanity check: the session exists. Fail loud — silent
    // grants against ghost sessions would confuse the operator
    // and accumulate dead rows.
    if store.get_session(sid).await?.is_none() {
        return Err(ArgoError::Provider(format!(
            "no session with id '{session_id_str}'"
        )));
    }
    let grant_id = store
        .add_session_grant(sid, tool, "cli", note)
        .await?;

    // Audit row so the chain shows the operator's intent. The
    // payload mirrors the rejection payload from #660 so a
    // verifier can correlate (escalation.required emitted, then
    // approval.granted with the same tool name).
    let audit = AuditSink::new(store.clone());
    let payload = serde_json::json!({
        "tool": tool,
        "grant_id": grant_id.to_string(),
        "source": "cli",
        "note": note,
    });
    audit
        .write(Some(sid), AuditKind::ApprovalGranted, payload)
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "grant_id": grant_id.to_string(),
                "session_id": session_id_str,
                "tool": tool,
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "approved tool '{tool}' for session '{session_id_str}' (grant {grant_id})"
        );
        println!("  → next turn against this session will let '{tool}' dispatch");
    }
    Ok(())
}

/// `argo session revoke-approval <session-id> --tool <name>`.
pub async fn revoke_approval(
    session_id_str: &str,
    tool: &str,
    json: bool,
) -> ArgoResult<()> {
    let sid = parse_session_id(session_id_str)?;
    let store = open_store().await?;
    let removed = store.revoke_session_grants_for_tool(sid, tool).await?;
    let audit = AuditSink::new(store.clone());
    let payload = serde_json::json!({
        "tool": tool,
        "removed": removed,
        "source": "cli",
    });
    audit
        .write(Some(sid), AuditKind::ApprovalRevoked, payload)
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "removed": removed,
                "session_id": session_id_str,
                "tool": tool,
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "revoked {removed} grant(s) for tool '{tool}' on session '{session_id_str}'"
        );
    }
    Ok(())
}
