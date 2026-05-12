//! `system:skill.quality_recompute` — daily 3am UTC quality refresh.
//!
//! Walks every skill version exposed by the caller-supplied
//! [`SkillCatalog`], rebuilds [`argo_skill::quality::QualityInputs`]
//! from a recent observation window (default 14 days), and writes
//! the recomputed `quality_score` back via [`QualityWriter`].
//!
//! ### Timebox
//!
//! `MAX_SKILLS_PER_TICK = 200` keeps a single tick under a minute on
//! a busy workspace. Operators may bump the limit via constructor;
//! the default matches the brief's stated bound.
//!
//! ### Idempotency
//!
//! `SkillQualityStore::record` uses `INSERT OR REPLACE` keyed on
//! `(skill_id, computed_at)`. Re-running this handler within the
//! same tick rewrites the rows in place without drift.
use argo_cron::{SystemJobContext, SystemJobHandler, SystemJobOutcome};
use argo_types::{ArgoResult, SkillId};
use async_trait::async_trait;
use std::sync::Arc;

/// Default cap on skills processed per tick.
pub const MAX_SKILLS_PER_TICK: usize = 200;

/// One catalog row reported by [`SkillCatalog::list_active`].
#[derive(Debug, Clone)]
pub struct CatalogSkill {
    /// Skill identifier (used as the persistence key).
    pub id: SkillId,
    /// Canonical skill name (used in summaries / error messages).
    pub name: String,
}

/// Pluggable seam over "every active skill". The cron daemon's real
/// catalog walks the skill_versions table; tests inject a fixed list.
#[async_trait]
pub trait SkillCatalog: Send + Sync {
    /// Return up to `limit` skills. Stable ordering is not required
    /// but encouraged so retries see the same prefix.
    async fn list_active(&self, limit: usize) -> ArgoResult<Vec<CatalogSkill>>;
}

/// Pluggable seam over "recompute + persist this skill's quality
/// score". The cron daemon's real writer is a `SkillQualityStore`;
/// tests use a counter.
#[async_trait]
pub trait QualityWriter: Send + Sync {
    /// Recompute + persist one skill's score. Returning `Ok(())`
    /// counts as a processed sub-task; `Err` is added to the
    /// outcome's `errors`.
    async fn recompute(&self, skill: &CatalogSkill) -> ArgoResult<()>;
}

/// Skill-quality recompute handler.
pub struct SkillQualityRecomputeHandler {
    catalog: Arc<dyn SkillCatalog>,
    writer: Arc<dyn QualityWriter>,
    limit: usize,
}

impl SkillQualityRecomputeHandler {
    /// Build with the default per-tick cap.
    #[must_use]
    pub fn new(catalog: Arc<dyn SkillCatalog>, writer: Arc<dyn QualityWriter>) -> Self {
        Self::with_limit(catalog, writer, MAX_SKILLS_PER_TICK)
    }

    /// Build with a custom limit (tests use this).
    #[must_use]
    pub fn with_limit(
        catalog: Arc<dyn SkillCatalog>,
        writer: Arc<dyn QualityWriter>,
        limit: usize,
    ) -> Self {
        Self {
            catalog,
            writer,
            limit,
        }
    }
}

#[async_trait]
impl SystemJobHandler for SkillQualityRecomputeHandler {
    async fn run(&self, _ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome> {
        let skills = self.catalog.list_active(self.limit).await?;
        if skills.is_empty() {
            return Ok(SystemJobOutcome::empty(
                "skill.quality_recompute: catalog empty",
            ));
        }
        let total = skills.len();
        let mut processed = 0_usize;
        let mut errors: Vec<String> = Vec::new();
        for s in &skills {
            match self.writer.recompute(s).await {
                Ok(()) => processed += 1,
                Err(e) => errors.push(format!("{} ({}): {e}", s.name, s.id)),
            }
        }
        Ok(SystemJobOutcome {
            jobs_processed: processed,
            skipped: 0,
            errors: errors.clone(),
            summary: format!(
                "skill.quality_recompute: {}/{} recomputed, {} errors",
                processed,
                total,
                errors.len()
            ),
        })
    }
}

/// No-op catalog returning an empty list — default wiring for
/// installs that haven't registered a real catalog yet.
pub struct NoopCatalog;

#[async_trait]
impl SkillCatalog for NoopCatalog {
    async fn list_active(&self, _limit: usize) -> ArgoResult<Vec<CatalogSkill>> {
        Ok(Vec::new())
    }
}

/// No-op quality writer.
pub struct NoopWriter;

#[async_trait]
impl QualityWriter for NoopWriter {
    async fn recompute(&self, _skill: &CatalogSkill) -> ArgoResult<()> {
        Ok(())
    }
}

/// Build a no-op handler — useful as the default `init-system`
/// wiring; operators register a real handler once their store is
/// open.
#[must_use]
pub fn noop_handler() -> Arc<dyn SystemJobHandler> {
    Arc::new(SkillQualityRecomputeHandler::new(
        Arc::new(NoopCatalog),
        Arc::new(NoopWriter),
    ))
}
