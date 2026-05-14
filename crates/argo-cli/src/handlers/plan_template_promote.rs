//! `argo plan template promote <id>` — flip a Dreams-distilled
//! learned plan template's lifecycle from Pending to Promoted.
//! TemplatePlanner consults promoted rows ahead of the shipped
//! defaults on the next `argo plan generate` call.
//!
//! Mission `mission-learned-plan-template-trust-ladder`: the
//! promote call also mirrors the row into the skill catalog (at
//! `argo-skill::trust::TrustGrade::Tenant`) via the concrete
//! [`SkillStoreSink`] defined below, so the template participates
//! in the trust ladder that gates skill surfacing.
use std::sync::Arc;

use argo_skill::ingest_learned::{demote_learned_template, from_learned_plan_template};
use argo_skill::trust::TrustGrade;
use argo_skill::SkillStore;
use argo_store::learned_plan_template::{
    get, promote, LearnedPlanTemplateRow, SkillIngestSink, TrustGrade as LifecycleTrustGrade,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;

use crate::home::{db_path, ensure_dir, resolve_home};

/// Concrete [`SkillIngestSink`] used by the CLI. Owns an
/// `Arc<SessionStore>` so it can build a fresh `SkillStore` per
/// call without cloning the underlying connection pool. The trait
/// indirection lives in `argo-store`; this impl bridges the
/// dependency back to `argo-skill` without a cycle.
pub struct SkillStoreSink {
    session: Arc<SessionStore>,
}

impl SkillStoreSink {
    /// Build a sink over an already-migrated session store.
    #[must_use]
    pub fn new(session: Arc<SessionStore>) -> Self {
        Self { session }
    }
}

#[async_trait]
impl SkillIngestSink for SkillStoreSink {
    async fn ingest_from_template(&self, row: &LearnedPlanTemplateRow) -> ArgoResult<()> {
        let skills = SkillStore::new(Arc::clone(&self.session));
        from_learned_plan_template(row, &skills, TrustGrade::Tenant)
            .await
            .map(|_| ())
    }
    async fn demote_from_template(&self, row: &LearnedPlanTemplateRow) -> ArgoResult<()> {
        let skills = SkillStore::new(Arc::clone(&self.session));
        demote_learned_template(row, &skills).await.map(|_| ())
    }
}

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let session = Arc::new(SessionStore::open(&db_path(&home)).await?);
    session.migrate().await?;

    let existing = get(session.pool(), id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no learned plan template with id '{id}'")))?;

    if existing.trust_grade == LifecycleTrustGrade::Promoted {
        if json {
            let envelope = serde_json::json!({
                "ok": true,
                "id": id,
                "already_promoted": true,
            });
            println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
        } else {
            println!("learned-plan-template {id} is already promoted");
        }
        return Ok(());
    }

    let sink = SkillStoreSink::new(Arc::clone(&session));
    promote(session.pool(), id, &sink).await?;
    let after = get(session.pool(), id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no learned plan template with id '{id}' after promote")))?;

    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "id": id,
            "row": after,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else {
        println!("learned-plan-template {id} promoted ({})", after.name);
        if let Some(p) = after.promoted_at {
            println!("  promoted_at : {p}");
        }
    }
    Ok(())
}
