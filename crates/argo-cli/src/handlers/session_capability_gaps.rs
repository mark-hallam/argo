//! `argo session capability-gaps <id>` — surface the #682
//! capability-gap detector to operators (#685, work order #4
//! follow-up).
//!
//! Today runs the policy-denial detector; future detectors
//! (manual_correction, weak_evidence, missing_knowledge,
//! repeated_similar_task) plug in here as they exist. The
//! detector NEVER auto-bypasses policy — it surfaces signals
//! so a human can decide.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::detect_policy_denials;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;
use uuid::Uuid;

pub async fn run(
    session_id_str: &str,
    since_minutes: i64,
    threshold: u64,
    json: bool,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let store = Arc::new(store);

    let uuid = Uuid::parse_str(session_id_str).map_err(|e| {
        ArgoError::Schema(format!("invalid session id '{session_id_str}': {e}"))
    })?;
    let sid = SessionId(uuid);

    // Fail loud on ghost session id — operator probably typo'd.
    if store.get_session(sid).await?.is_none() {
        return Err(ArgoError::Provider(format!(
            "no session with id '{session_id_str}'"
        )));
    }

    let denials = detect_policy_denials(&store, sid, since_minutes, threshold).await?;

    if json {
        let arr: Vec<serde_json::Value> = denials.iter().map(|g| g.to_json()).collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "session_id": session_id_str,
                "since_minutes": since_minutes,
                "threshold": threshold,
                "count": arr.len(),
                "gaps": arr,
            }))
            .unwrap_or_default()
        );
    } else if denials.is_empty() {
        let scope = if since_minutes > 0 {
            format!("last {since_minutes} min")
        } else {
            "session".into()
        };
        println!(
            "session '{session_id_str}': no capability gaps in {scope} (threshold ≥ {threshold})"
        );
    } else {
        println!(
            "session '{session_id_str}' — {} capability gap(s):",
            denials.len()
        );
        for g in &denials {
            println!("  [{}] {} ({}× ) — {}", g.kind.label(), g.anchor, g.count, g.message);
        }
    }
    Ok(())
}
