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

/// `argo session approve <session-id> --tool <name> [--note
/// "<text>"] [--once]`.
pub async fn approve(
    session_id_str: &str,
    tool: &str,
    note: Option<&str>,
    once: bool,
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
        .add_session_grant(sid, tool, "cli", note, once)
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
        "one_shot": once,
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
                "one_shot": once,
            }))
            .unwrap_or_default()
        );
    } else {
        let suffix = if once { " (one-shot)" } else { "" };
        println!(
            "approved tool '{tool}' for session '{session_id_str}'{suffix} (grant {grant_id})"
        );
        if once {
            println!(
                "  → next dispatch of '{tool}' goes through; subsequent calls re-block"
            );
        } else {
            println!(
                "  → next turn against this session will let '{tool}' dispatch"
            );
        }
    }
    Ok(())
}

/// `argo session list-grants <session-id> [--json]`. Prints
/// every active grant for a session — useful for "what's
/// currently allowed through the escalation gate?" (#666).
pub async fn list_grants(session_id_str: &str, json: bool) -> ArgoResult<()> {
    let sid = parse_session_id(session_id_str)?;
    let store = open_store().await?;
    if store.get_session(sid).await?.is_none() {
        return Err(ArgoError::Provider(format!(
            "no session with id '{session_id_str}'"
        )));
    }
    let grants = store.list_session_grants(sid).await?;
    if json {
        let arr: Vec<serde_json::Value> = grants
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": g.id.to_string(),
                    "tool": g.tool_name,
                    "granted_at_secs": g.granted_at_secs,
                    "source": g.source,
                    "note": g.notes,
                    "one_shot": g.is_one_shot,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "session_id": session_id_str,
                "count": arr.len(),
                "grants": arr,
            }))
            .unwrap_or_default()
        );
    } else if grants.is_empty() {
        println!("(no active grants on session '{session_id_str}')");
    } else {
        println!(
            "session '{session_id_str}' — {} active grant(s)",
            grants.len()
        );
        for g in &grants {
            let note = g
                .notes
                .as_deref()
                .filter(|n| !n.is_empty())
                .map(|n| format!(" — {n}"))
                .unwrap_or_default();
            let kind = if g.is_one_shot { "one-shot" } else { "persistent" };
            println!(
                "  [{}] tool={} ({kind}, source={}, granted_at_secs={}){note}",
                g.id, g.tool_name, g.source, g.granted_at_secs
            );
        }
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
