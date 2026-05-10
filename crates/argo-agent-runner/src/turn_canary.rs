//! Per-skill canary routing for claude-skill-canary-deployment.
//!
//! Loaded via `#[path = "turn_canary.rs"] mod turn_canary;` from
//! `turn.rs` — kept in a sibling file because `turn.rs` is at the
//! 500-LOC ceiling. Reads `skill_canary` for each active skill,
//! writes a `RoutingDecision` audit row tagged
//! `kind: "canary_pick"`, and substitutes the canary body when the
//! seeded pick lands canary-side. No canary / expired = no-op.
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;

pub(crate) async fn apply_canary_routing(
    store: &Arc<SessionStore>,
    skills: &argo_skill::SkillStore,
    session_id: SessionId,
    active: &mut [argo_skill::Skill],
) -> ArgoResult<()> {
    let canary = argo_skill::CanaryStore::new(Arc::clone(store));
    let canary_audit = AuditSink::new(Arc::clone(store));
    let now_micros = chrono::Utc::now().timestamp_micros();
    for s in active.iter_mut() {
        let name = &s.frontmatter.name;
        let seed = argo_skill::seed_for_session_skill(session_id, name);
        let Some(d) = argo_skill::pick_canary_for_skill(&canary, name, seed, now_micros).await?
        else {
            continue;
        };
        canary_audit
            .write(
                Some(session_id),
                AuditKind::RoutingDecision,
                serde_json::json!({
                    "kind": "canary_pick",
                    "skill_id": name,
                    "baseline_version": d.baseline_version,
                    "canary_version": d.canary_version,
                    "chosen_version": d.chosen_version,
                    "traffic_percent": d.traffic_percent,
                    "seed_hash": d.seed_hash,
                }),
                None,
            )
            .await?;
        if d.chose_canary {
            let body = argo_skill::load_canary_body(skills, name, &d.chosen_version)
                .await?
                .ok_or_else(|| {
                    ArgoError::Schema(format!(
                        "canary: skill '{name}' has no version '{}'",
                        d.chosen_version
                    ))
                })?;
            s.body = body;
        }
    }
    Ok(())
}
