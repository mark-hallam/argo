//! `argo session attach-skill / detach-skill / skills` —
//! manage the per-session skill set introduced in v4 schema
//! (#279). Lives in its own module so `session.rs` stays under
//! the 500-LOC ceiling.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;

pub(crate) async fn attach(raw_session: &str, skill_name: &str) -> ArgoResult<()> {
    let (store, sid, skills) = open_handles(raw_session).await?;
    let target = skills
        .find_active_by_name(skill_name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{skill_name}'")))?;
    skills.attach_to_session(sid, target.id).await?;
    println!(
        "argo session attach-skill: {raw_session} ← {skill_name} v{}",
        target.frontmatter.version,
    );
    let _ = store; // keep the Arc alive for the println scope
    Ok(())
}

pub(crate) async fn detach(raw_session: &str, skill_name: &str) -> ArgoResult<()> {
    let (store, sid, skills) = open_handles(raw_session).await?;
    let target = skills
        .find_active_by_name(skill_name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{skill_name}'")))?;
    let removed = skills.detach_from_session(sid, target.id).await?;
    if removed == 0 {
        return Err(ArgoError::Provider(format!(
            "skill '{skill_name}' was not attached to session {raw_session}"
        )));
    }
    println!("argo session detach-skill: {raw_session} → removed {skill_name}");
    let _ = store;
    Ok(())
}

/// `argo session clear-skills <id>` — detach every attached
/// skill from a session. Mirrors
/// `POST /sessions/<id>/skills/clear`. Useful for resetting to
/// the fall-through-all-active-skills loader behaviour without
/// pre-listing the attached set.
pub(crate) async fn clear(raw_session: &str) -> ArgoResult<()> {
    let (_, sid, skills) = open_handles(raw_session).await?;
    let attached = skills.skills_for_session(sid).await?;
    let mut detached: u64 = 0;
    for s in attached {
        detached += skills.detach_from_session(sid, s.id).await?;
    }
    println!(
        "argo session clear-skills: {raw_session} → detached {detached}"
    );
    Ok(())
}

pub(crate) async fn list(raw_session: &str) -> ArgoResult<()> {
    let (_, sid, skills) = open_handles(raw_session).await?;
    let attached = skills.skills_for_session(sid).await?;
    if attached.is_empty() {
        println!(
            "(no skills attached to {raw_session} — agent loop sees every active skill)"
        );
        return Ok(());
    }
    println!("attached skills ({}):", attached.len());
    for s in attached {
        println!(
            "  {:<24} v{:<8} {:?}",
            s.frontmatter.name, s.frontmatter.version, s.frontmatter.lock,
        );
    }
    Ok(())
}

async fn open_handles(
    raw_session: &str,
) -> ArgoResult<(Arc<SessionStore>, SessionId, SkillStore)> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let sid = super::session_resolve::resolve(&store, raw_session).await?;
    let skills = SkillStore::new(Arc::clone(&store));
    Ok((store, sid, skills))
}
