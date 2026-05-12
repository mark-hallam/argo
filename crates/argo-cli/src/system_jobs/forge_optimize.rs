//! `system:forge.optimize` — every-6h Forge re-optimize tick.
//!
//! Selects the `MAX_STALE_PER_TICK` (default 5) stalest skills by
//! `(trust_grade ASC, last_forge_at ASC)` and invokes the
//! caller-supplied [`ForgeOptimizeInvoker`] against each. The
//! handler is parameterised over [`StalestSkillSelector`] and
//! [`ForgeOptimizeInvoker`] so the cron daemon can wire real
//! dependencies at boot while tests use simple in-memory stubs.
//!
//! ### Idempotency
//!
//! Re-running the same tick is safe — the selector either returns
//! the same stale set (if the previous tick's forge runs are still
//! in flight or failed) or a freshly-stale set (if previous runs
//! succeeded and bumped `last_forge_at`).
//!
//! ### Timebox
//!
//! `MAX_STALE_PER_TICK = 5` keeps a single tick's wall-clock bounded
//! (each forge run is minutes-scale). Operators wanting a deeper
//! sweep run `argo cron trigger system:forge.optimize` repeatedly.
use argo_cron::{SystemJobContext, SystemJobHandler, SystemJobOutcome};
use argo_types::ArgoResult;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Default cap on skills processed per tick.
pub const MAX_STALE_PER_TICK: usize = 5;

/// One staleness candidate produced by [`StalestSkillSelector`].
#[derive(Debug, Clone)]
pub struct StaleSkill {
    /// Canonical skill name.
    pub name: String,
    /// Active version string at selection time.
    pub version: String,
    /// Trust grade in `[0.0, 1.0]`. Lower = staler.
    pub trust_grade: f64,
    /// Most recent forge-run timestamp; `None` = never forged.
    pub last_forge_at: Option<DateTime<Utc>>,
}

/// Pluggable seam over "the N stalest skills". The cron daemon's
/// real selector queries the skill catalog + forge_run history; the
/// in-memory stub used in tests just returns a fixed list.
#[async_trait]
pub trait StalestSkillSelector: Send + Sync {
    /// Return up to `limit` stalest skills, sorted stalest-first.
    async fn select(&self, limit: usize) -> ArgoResult<Vec<StaleSkill>>;
}

/// Pluggable seam over "run forge optimize-judge against this
/// skill". The cron daemon's real invoker spawns the existing
/// `forge optimize-judge` codepath; tests use a counter.
#[async_trait]
pub trait ForgeOptimizeInvoker: Send + Sync {
    /// Optimize one skill. Returning `Ok(())` counts as a processed
    /// sub-task; `Err` is added to `outcome.errors`.
    async fn optimize_one(&self, skill: &StaleSkill) -> ArgoResult<()>;
}

/// Forge-optimize tick handler.
pub struct ForgeOptimizeHandler {
    selector: Arc<dyn StalestSkillSelector>,
    invoker: Arc<dyn ForgeOptimizeInvoker>,
    limit: usize,
}

impl ForgeOptimizeHandler {
    /// Build a handler with the default per-tick cap.
    #[must_use]
    pub fn new(
        selector: Arc<dyn StalestSkillSelector>,
        invoker: Arc<dyn ForgeOptimizeInvoker>,
    ) -> Self {
        Self::with_limit(selector, invoker, MAX_STALE_PER_TICK)
    }

    /// Build a handler with a custom limit (tests use this).
    #[must_use]
    pub fn with_limit(
        selector: Arc<dyn StalestSkillSelector>,
        invoker: Arc<dyn ForgeOptimizeInvoker>,
        limit: usize,
    ) -> Self {
        Self {
            selector,
            invoker,
            limit,
        }
    }
}

#[async_trait]
impl SystemJobHandler for ForgeOptimizeHandler {
    async fn run(&self, _ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome> {
        let stale = self.selector.select(self.limit).await?;
        if stale.is_empty() {
            return Ok(SystemJobOutcome::empty(
                "forge.optimize: no stale skills found",
            ));
        }
        let total = stale.len();
        let mut processed = 0_usize;
        let mut errors: Vec<String> = Vec::new();
        for s in &stale {
            match self.invoker.optimize_one(s).await {
                Ok(()) => processed += 1,
                Err(e) => errors.push(format!(
                    "{}@{} (trust={:.2}, last_forge={:?}): {e}",
                    s.name, s.version, s.trust_grade, s.last_forge_at
                )),
            }
        }
        let summary = format!(
            "forge.optimize: {}/{} skills processed, {} errors",
            processed,
            total,
            errors.len()
        );
        Ok(SystemJobOutcome {
            jobs_processed: processed,
            skipped: 0,
            errors,
            summary,
        })
    }
}

/// No-op selector returning the empty list — used by the default
/// cron-handler wiring when no production selector has been
/// registered.
pub struct NoopSelector;

#[async_trait]
impl StalestSkillSelector for NoopSelector {
    async fn select(&self, _limit: usize) -> ArgoResult<Vec<StaleSkill>> {
        Ok(Vec::new())
    }
}

/// No-op invoker returning `Ok(())` — paired with [`NoopSelector`]
/// it makes the handler a structured "nothing to do" reporter that
/// still goes through the dispatch path.
pub struct NoopInvoker;

#[async_trait]
impl ForgeOptimizeInvoker for NoopInvoker {
    async fn optimize_one(&self, _skill: &StaleSkill) -> ArgoResult<()> {
        Ok(())
    }
}

/// Convenience: build a handler that performs no work — every tick
/// returns the empty `SystemJobOutcome`. Used by the default
/// cron-handler wiring; operators register a real handler via the
/// follow-up "wire real forge optimize selector" brief.
#[must_use]
pub fn noop_handler() -> Arc<dyn SystemJobHandler> {
    Arc::new(ForgeOptimizeHandler::new(
        Arc::new(NoopSelector),
        Arc::new(NoopInvoker),
    ))
}
