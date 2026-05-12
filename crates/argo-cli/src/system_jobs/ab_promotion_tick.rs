//! `system:ab.promotion` — weekly Sun 5am UTC A/B sweep.
//!
//! Queries A/B experiments via [`AbExperimentStore`], computes the
//! statistical decision per past-min-sample experiment via
//! [`AbDecider`], and dispatches the resulting [`AbDecision`] back
//! to the store (promote / extend / terminate).
//!
//! ### Timebox
//!
//! `MAX_EXPERIMENTS_PER_TICK = 50` caps a single tick at 50 decisions.
//! Operators trigger an extra sweep via `argo cron trigger
//! system:ab.promotion` when needed.
//!
//! ### Conservative bias
//!
//! Per the brief: the handler extends when min-sample crossed but
//! the confidence interval still overlaps; promote only when CI is
//! fully separated; terminate only when statistical futility is
//! detected by the decider. The handler itself owns no statistics —
//! that judgement lives in [`AbDecider::decide`].
use argo_cron::{SystemJobContext, SystemJobHandler, SystemJobOutcome};
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::sync::Arc;

/// Default cap on experiments processed per tick.
pub const MAX_EXPERIMENTS_PER_TICK: usize = 50;

/// One A/B experiment row observed by the handler.
#[derive(Debug, Clone)]
pub struct AbExperiment {
    /// Experiment id (canonical string).
    pub id: String,
    /// Skill being A/B tested.
    pub skill_name: String,
    /// Total trials accumulated so far.
    pub trials: usize,
    /// Minimum-sample threshold for any decision.
    pub min_sample: usize,
}

impl AbExperiment {
    /// `true` when the experiment has cleared its min-sample
    /// threshold and is therefore eligible for a decision.
    #[must_use]
    pub fn eligible(&self) -> bool {
        self.trials >= self.min_sample
    }
}

/// Decision produced by [`AbDecider::decide`].
///
/// The `Promote` and `Terminate` variants are part of the public
/// API (real deciders return them); the shipped default decider
/// returns `Extend` only. `#[allow(dead_code)]` on those two
/// variants suppresses the "never constructed in production"
/// dead-code lint while keeping them visible to tests + future
/// decider impls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbDecision {
    /// Promote the winning variant; conclude the experiment.
    #[allow(dead_code)]
    Promote {
        /// Winning variant id.
        variant: String,
    },
    /// Keep running — CI still overlaps but min-sample crossed.
    Extend,
    /// Terminate without promoting (futility / regression).
    #[allow(dead_code)]
    Terminate {
        /// Operator-readable reason ("regression detected", ...).
        reason: String,
    },
}

impl AbDecision {
    /// One-word label used in summary lines.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Promote { variant } => format!("promote({variant})"),
            Self::Extend => "extend".to_string(),
            Self::Terminate { reason } => format!("terminate({reason})"),
        }
    }
}

/// Pluggable seam over "every A/B experiment". The cron daemon
/// queries the variant_store; tests inject a fixed list.
#[async_trait]
pub trait AbExperimentStore: Send + Sync {
    /// List up to `limit` experiments (any state).
    async fn list(&self, limit: usize) -> ArgoResult<Vec<AbExperiment>>;
    /// Apply a decision to one experiment. Promote / Extend /
    /// Terminate semantics live in the variant_store impl.
    async fn apply(&self, experiment: &AbExperiment, decision: &AbDecision) -> ArgoResult<()>;
}

/// Pluggable seam over "compute the statistical decision for this
/// experiment". The cron daemon's real decider wraps the existing
/// stats helpers (`argo_skill::ab_test_stats`); tests inject a
/// fixed mapping.
#[async_trait]
pub trait AbDecider: Send + Sync {
    /// Return the per-experiment decision.
    async fn decide(&self, experiment: &AbExperiment) -> ArgoResult<AbDecision>;
}

/// A/B promotion-tick handler.
pub struct AbPromotionHandler {
    store: Arc<dyn AbExperimentStore>,
    decider: Arc<dyn AbDecider>,
    limit: usize,
}

impl AbPromotionHandler {
    /// Build with the default per-tick cap.
    #[must_use]
    pub fn new(store: Arc<dyn AbExperimentStore>, decider: Arc<dyn AbDecider>) -> Self {
        Self::with_limit(store, decider, MAX_EXPERIMENTS_PER_TICK)
    }

    /// Build with a custom limit (tests use this).
    #[must_use]
    pub fn with_limit(
        store: Arc<dyn AbExperimentStore>,
        decider: Arc<dyn AbDecider>,
        limit: usize,
    ) -> Self {
        Self {
            store,
            decider,
            limit,
        }
    }
}

#[async_trait]
impl SystemJobHandler for AbPromotionHandler {
    async fn run(&self, _ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome> {
        let experiments = self.store.list(self.limit).await?;
        if experiments.is_empty() {
            return Ok(SystemJobOutcome::empty(
                "ab.promotion: no experiments registered",
            ));
        }
        let mut processed = 0_usize;
        let mut skipped = 0_usize;
        let mut errors: Vec<String> = Vec::new();
        for exp in &experiments {
            if !exp.eligible() {
                skipped += 1;
                continue;
            }
            let decision = match self.decider.decide(exp).await {
                Ok(d) => d,
                Err(e) => {
                    errors.push(format!("{}: decide: {e}", exp.id));
                    continue;
                }
            };
            if let Err(e) = self.store.apply(exp, &decision).await {
                errors.push(format!(
                    "{} ({}): apply({}): {e}",
                    exp.id,
                    exp.skill_name,
                    decision.label()
                ));
                continue;
            }
            processed += 1;
        }
        Ok(SystemJobOutcome {
            jobs_processed: processed,
            skipped,
            errors: errors.clone(),
            summary: format!(
                "ab.promotion: {} decided / {} below min-sample / {} errors (of {} experiments)",
                processed,
                skipped,
                errors.len(),
                experiments.len()
            ),
        })
    }
}

/// No-op A/B store — default wiring; real variant_store is wired
/// by a follow-up brief.
pub struct NoopStore;

#[async_trait]
impl AbExperimentStore for NoopStore {
    async fn list(&self, _limit: usize) -> ArgoResult<Vec<AbExperiment>> {
        Ok(Vec::new())
    }
    async fn apply(&self, _exp: &AbExperiment, _d: &AbDecision) -> ArgoResult<()> {
        Ok(())
    }
}

/// No-op decider. Paired with [`NoopStore`] makes the default
/// handler a structured "nothing to do" reporter.
pub struct NoopDecider;

#[async_trait]
impl AbDecider for NoopDecider {
    async fn decide(&self, _exp: &AbExperiment) -> ArgoResult<AbDecision> {
        Ok(AbDecision::Extend)
    }
}

/// Build a no-op handler for default init-system wiring.
#[must_use]
pub fn noop_handler() -> Arc<dyn SystemJobHandler> {
    Arc::new(AbPromotionHandler::new(
        Arc::new(NoopStore),
        Arc::new(NoopDecider),
    ))
}
