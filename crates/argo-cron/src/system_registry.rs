//! System-job handler registry (`mission-cron-tick-maintenance-jobs`).
//!
//! Workspace-global maintenance jobs (Forge optimize, skill quality
//! recompute, Dreams curation, A/B promotion) share the cron tick
//! loop with operator-defined jobs but dispatch through an internal
//! registry rather than an output-delivery adapter. The
//! [`SystemJobRegistry`] maps a system job name (e.g.
//! `"forge.optimize"`) to a [`SystemJobHandler`] impl. The cron
//! tick handler (`argo-cli::handlers::cron`) looks up
//! [`crate::system_job_name`] on each due job and, if non-`None`,
//! dispatches via the registry.
//!
//! The trait is intentionally minimal — `async fn run(ctx) -> Outcome`
//! — so future v2 system jobs (`system:audit.chain_watchdog`,
//! `system:trust_observation.gc`, ...) plug in without ABI churn.
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Per-tick context handed to a [`SystemJobHandler::run`] call. The
/// workspace root lets handlers locate config / data on disk; the
/// `now` field is supplied so unit tests can drive deterministic
/// schedules.
#[derive(Debug, Clone)]
pub struct SystemJobContext {
    /// Workspace root (the directory that contains `cron/jobs/`).
    pub workspace_root: PathBuf,
    /// Logical "current time" for this tick. Real callers pass
    /// `chrono::Utc::now()`; tests pin a fixed timestamp.
    pub now: chrono::DateTime<chrono::Utc>,
}

impl SystemJobContext {
    /// Build a context with the given workspace root + wall-clock
    /// "now".
    #[must_use]
    pub fn new(workspace_root: PathBuf, now: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            workspace_root,
            now,
        }
    }

    /// Convenience constructor using `chrono::Utc::now()`.
    #[must_use]
    pub fn at_now(workspace_root: PathBuf) -> Self {
        Self::new(workspace_root, chrono::Utc::now())
    }
}

/// Structured outcome of one system job run. Handlers report partial
/// failures via [`Self::errors`] rather than propagating a single
/// `Err` for the whole run — operators want to see "3 succeeded, 2
/// failed with X" in the daemon log, not just the first failure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemJobOutcome {
    /// Sub-tasks the handler processed successfully.
    pub jobs_processed: usize,
    /// Sub-tasks skipped (idempotent re-runs, below threshold,
    /// nothing to do).
    pub skipped: usize,
    /// Per-sub-task error strings (empty on full success). One
    /// entry per failed sub-task.
    pub errors: Vec<String>,
    /// One-line human-readable summary, suitable for stderr/tracing.
    pub summary: String,
}

impl SystemJobOutcome {
    /// Build an outcome with all-zero counters and the given
    /// summary string. Useful for "nothing to do" returns.
    #[must_use]
    pub fn empty(summary: impl Into<String>) -> Self {
        Self {
            jobs_processed: 0,
            skipped: 0,
            errors: Vec::new(),
            summary: summary.into(),
        }
    }
}

/// Trait implemented by every system maintenance job. The cron
/// tick loop looks up the registered handler by system-job name
/// and awaits [`Self::run`]; the returned outcome is emitted on
/// stderr (and as structured tracing) by the surrounding daemon.
#[async_trait]
pub trait SystemJobHandler: Send + Sync {
    /// Execute one tick. Returning `Err` indicates the entire
    /// handler failed (e.g. couldn't open the store); per-sub-task
    /// errors should be reported via [`SystemJobOutcome::errors`].
    async fn run(&self, ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome>;
}

/// Registry of system-job names to handlers. Cheap to clone — the
/// inner map is `Arc`-shared.
#[derive(Default, Clone)]
pub struct SystemJobRegistry {
    handlers: Arc<HashMap<&'static str, Arc<dyn SystemJobHandler>>>,
}

impl std::fmt::Debug for SystemJobRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemJobRegistry")
            .field("names", &self.names())
            .finish()
    }
}

/// Builder for [`SystemJobRegistry`].
#[derive(Default)]
pub struct SystemJobRegistryBuilder {
    handlers: HashMap<&'static str, Arc<dyn SystemJobHandler>>,
}

impl SystemJobRegistryBuilder {
    /// Empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for `name`. Re-registering the same name
    /// overwrites the previous binding (last-writer-wins, matching
    /// the file-store contract on `(tenant, name)`).
    #[must_use]
    pub fn register(mut self, name: &'static str, handler: Arc<dyn SystemJobHandler>) -> Self {
        self.handlers.insert(name, handler);
        self
    }

    /// Freeze into an [`SystemJobRegistry`].
    #[must_use]
    pub fn build(self) -> SystemJobRegistry {
        SystemJobRegistry {
            handlers: Arc::new(self.handlers),
        }
    }
}

impl SystemJobRegistry {
    /// Empty registry — no system jobs registered. Useful as a
    /// fallback when the daemon hasn't wired any handlers yet.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Look up a handler by system-job name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn SystemJobHandler>> {
        self.handlers.get(name).cloned()
    }

    /// Names of every registered handler, sorted for deterministic
    /// output (used by `argo cron init-system` summaries + tests).
    #[must_use]
    pub fn names(&self) -> Vec<&'static str> {
        let mut v: Vec<_> = self.handlers.keys().copied().collect();
        v.sort_unstable();
        v
    }

    /// `true` if a handler is registered for `name`.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubHandler {
        outcome: SystemJobOutcome,
    }

    #[async_trait]
    impl SystemJobHandler for StubHandler {
        async fn run(&self, _ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome> {
            Ok(self.outcome.clone())
        }
    }

    #[tokio::test]
    async fn registry_dispatches_registered_handler() {
        let reg = SystemJobRegistryBuilder::new()
            .register(
                "demo.job",
                Arc::new(StubHandler {
                    outcome: SystemJobOutcome::empty("ok"),
                }),
            )
            .build();
        let handler = reg.get("demo.job").expect("registered");
        let ctx = SystemJobContext::at_now(PathBuf::from("/tmp"));
        let out = handler.run(ctx).await.unwrap();
        assert_eq!(out.summary, "ok");
    }

    #[test]
    fn empty_registry_misses_lookup() {
        let reg = SystemJobRegistry::empty();
        assert!(reg.get("nope").is_none());
        assert!(reg.names().is_empty());
        assert!(!reg.has("nope"));
    }

    #[test]
    fn names_are_sorted() {
        let reg = SystemJobRegistryBuilder::new()
            .register(
                "zeta",
                Arc::new(StubHandler {
                    outcome: SystemJobOutcome::empty(""),
                }),
            )
            .register(
                "alpha",
                Arc::new(StubHandler {
                    outcome: SystemJobOutcome::empty(""),
                }),
            )
            .build();
        assert_eq!(reg.names(), vec!["alpha", "zeta"]);
    }
}
