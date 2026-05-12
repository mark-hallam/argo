//! `system:dreams.curation_tick` — daily 4:30am UTC Dreams pass.
//!
//! Wraps the existing Dreams curation pipeline (template proposal +
//! distillation) over the last `WINDOW_HOURS` of accumulated agent
//! trajectories. The pluggable [`DreamsCurator`] seam lets the cron
//! daemon wire `argo_skill::dreams_distillation` /
//! `argo_runtime::dreams_plan_templates` at boot while tests use
//! in-memory stubs.
//!
//! ### Timebox
//!
//! `WINDOW_HOURS = 24` matches the daily cadence — every tick
//! processes one calendar day of trajectories.
//!
//! ### Idempotency
//!
//! The handler is idempotent at the SystemJobOutcome level: re-running
//! against the same window produces a deterministic outcome. The
//! underlying distillation persists template proposals with content
//! hashes, so re-processing a trajectory does not duplicate templates.
use argo_cron::{SystemJobContext, SystemJobHandler, SystemJobOutcome};
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::sync::Arc;

/// Default trajectory window (hours).
pub const WINDOW_HOURS: i64 = 24;

/// Outcome counters produced by [`DreamsCurator::curate`]. Mirrors
/// the brief's `templates_proposed / templates_promoted /
/// templates_discarded` triple.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DreamsCurationReport {
    /// Templates proposed during this tick.
    pub templates_proposed: usize,
    /// Templates promoted to the canonical store.
    pub templates_promoted: usize,
    /// Templates rejected by the curation gate.
    pub templates_discarded: usize,
    /// Trajectories scanned (informational; not in `jobs_processed`).
    pub trajectories_scanned: usize,
}

/// Pluggable seam over the Dreams curation pipeline.
#[async_trait]
pub trait DreamsCurator: Send + Sync {
    /// Curate the last `window_hours` of trajectories.
    async fn curate(&self, window_hours: i64) -> ArgoResult<DreamsCurationReport>;
}

/// Dreams curation handler.
pub struct DreamsCurationHandler {
    curator: Arc<dyn DreamsCurator>,
    window_hours: i64,
}

impl DreamsCurationHandler {
    /// Build with the default 24-hour window.
    #[must_use]
    pub fn new(curator: Arc<dyn DreamsCurator>) -> Self {
        Self::with_window(curator, WINDOW_HOURS)
    }

    /// Build with a custom window (tests use this).
    #[must_use]
    pub fn with_window(curator: Arc<dyn DreamsCurator>, window_hours: i64) -> Self {
        Self {
            curator,
            window_hours,
        }
    }
}

#[async_trait]
impl SystemJobHandler for DreamsCurationHandler {
    async fn run(&self, _ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome> {
        let report = self.curator.curate(self.window_hours).await?;
        if report.trajectories_scanned == 0 && report.templates_proposed == 0 {
            return Ok(SystemJobOutcome::empty(
                "dreams.curation_tick: no trajectories in window",
            ));
        }
        let summary = format!(
            "dreams.curation_tick: scanned {} trajectories — \
             proposed {} / promoted {} / discarded {}",
            report.trajectories_scanned,
            report.templates_proposed,
            report.templates_promoted,
            report.templates_discarded,
        );
        Ok(SystemJobOutcome {
            jobs_processed: report.templates_promoted,
            skipped: report.templates_discarded,
            errors: Vec::new(),
            summary,
        })
    }
}

/// No-op curator returning the all-zero report. Default wiring;
/// operators register a real curator via a follow-up brief.
pub struct NoopCurator;

#[async_trait]
impl DreamsCurator for NoopCurator {
    async fn curate(&self, _window_hours: i64) -> ArgoResult<DreamsCurationReport> {
        Ok(DreamsCurationReport::default())
    }
}

/// Build a no-op handler for the default init-system wiring.
#[must_use]
pub fn noop_handler() -> Arc<dyn SystemJobHandler> {
    Arc::new(DreamsCurationHandler::new(Arc::new(NoopCurator)))
}
