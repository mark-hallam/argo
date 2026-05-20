//! Per-worker learning cron registry.
//!
//! Owns the set of per-worker [`spawn_per_worker_learning_cron`] handles
//! the gateway holds open while the process is running. Two entry
//! points produce enrollments:
//!
//! - boot enumerates `worker WHERE archived_at IS NULL` and enrolls
//!   each row through this registry (see
//!   `argo-cli/src/handlers/gateway/boot_serve.rs`).
//! - worker-hire (via [`IdentityStore::create_worker`]) invokes the
//!   same registry on each freshly committed worker, so workers
//!   created post-boot start participating in the learning loop on
//!   the next cron tick instead of waiting for a gateway restart.
//!
//! Idempotent: enrolling the same `worker_id` twice is a no-op (no
//! panic, no double-spawn). Construction parameters bind the
//! per-worker tick to a single [`SkillStore`], a base
//! [`PerWorkerLearningConfig`] (the registry stamps `tenant_id` per
//! enrollment), and a shared cron interval.
//!
//! Archive cleanup (aborting the handle when a worker is archived) is
//! deliberately out of scope for the
//! `mission-audit-skill-learning-dynamic-enroll` brief — a follow-up
//! brief revisits it once worker archival lifecycle is first-class.
//! Until then, an archived worker's cron keeps ticking against an
//! `archived_at IS NULL` filter on the source ratings, so the loop
//! produces no new candidates but does cost a periodic SQL probe.
//!
//! [`IdentityStore::create_worker`]: argo_identity::IdentityStore::create_worker

use crate::per_worker_learning_cron::{spawn_per_worker_learning_cron, PerWorkerLearningConfig};
use crate::store::SkillStore;
use argo_store::WorkerId;
use argo_types::TenantId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;

/// Set of live per-worker learning cron handles, keyed by [`WorkerId`].
///
/// Construct one per gateway boot; share via `Arc<Self>`. Each enrolled
/// worker spawns one tokio task that runs forever (until the process
/// exits or the handle is aborted on `Drop`).
///
/// ```no_run
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// use argo_skill::{PerWorkerLearningConfig, SkillStore, WorkerLearningRegistry};
/// use argo_store::{SessionStore, WorkerId};
/// use argo_types::TenantId;
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// let store = Arc::new(SessionStore::open(":memory:").await?);
/// let skills = Arc::new(SkillStore::new(store));
/// let registry = WorkerLearningRegistry::new(
///     skills,
///     PerWorkerLearningConfig::default(),
///     Duration::from_secs(60),
/// );
/// assert!(registry.enroll(WorkerId::new(), TenantId::new()));
/// # Ok(()) }
/// ```
pub struct WorkerLearningRegistry {
    skills: Arc<SkillStore>,
    base_config: PerWorkerLearningConfig,
    interval: Duration,
    handles: Mutex<HashMap<WorkerId, JoinHandle<()>>>,
}

impl WorkerLearningRegistry {
    /// Construct an empty registry bound to `skills`, the base
    /// `config` (the registry stamps `tenant_id` per enrollment so
    /// caller-supplied `config.tenant_id` is overwritten), and the
    /// cron tick `interval`.
    #[must_use]
    pub fn new(
        skills: Arc<SkillStore>,
        config: PerWorkerLearningConfig,
        interval: Duration,
    ) -> Self {
        Self {
            skills,
            base_config: config,
            interval,
            handles: Mutex::new(HashMap::new()),
        }
    }

    /// Enroll `worker_id` (scoped to `tenant_id`). Returns `true` iff
    /// a new cron handle was spawned; `false` if `worker_id` was
    /// already enrolled (no double-spawn). A poisoned lock returns
    /// `false` without panicking — the gateway favors degraded
    /// behavior over panicking the worker-hire path.
    pub fn enroll(&self, worker_id: WorkerId, tenant_id: TenantId) -> bool {
        let Ok(mut guard) = self.handles.lock() else {
            tracing::warn!(
                target: "argo_skill::per_worker_learning_registry",
                worker_id = %worker_id,
                "registry mutex poisoned; skipping enrollment",
            );
            return false;
        };
        if guard.contains_key(&worker_id) {
            return false;
        }
        let mut tick_cfg = self.base_config.clone();
        tick_cfg.tenant_id = Some(tenant_id);
        let handle = spawn_per_worker_learning_cron(
            worker_id,
            Arc::clone(&self.skills),
            tick_cfg,
            self.interval,
        );
        guard.insert(worker_id, handle);
        true
    }

    /// Whether `worker_id` currently holds a learning handle.
    ///
    /// ```no_run
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_skill::{PerWorkerLearningConfig, SkillStore, WorkerLearningRegistry};
    /// use argo_store::{SessionStore, WorkerId};
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// let store = Arc::new(SessionStore::open(":memory:").await?);
    /// let skills = Arc::new(SkillStore::new(store));
    /// let registry = WorkerLearningRegistry::new(
    ///     skills, PerWorkerLearningConfig::default(), Duration::from_secs(60),
    /// );
    /// assert!(!registry.is_enrolled(WorkerId::new()));
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn is_enrolled(&self, worker_id: WorkerId) -> bool {
        self.handles
            .lock()
            .map(|g| g.contains_key(&worker_id))
            .unwrap_or(false)
    }

    /// Number of workers currently enrolled.
    #[must_use]
    pub fn enrolled_count(&self) -> usize {
        self.handles.lock().map(|g| g.len()).unwrap_or(0)
    }
}

impl Drop for WorkerLearningRegistry {
    fn drop(&mut self) {
        if let Ok(guard) = self.handles.lock() {
            for handle in guard.values() {
                handle.abort();
            }
        }
    }
}

/// Convenience free function mirroring the registry method, kept for
/// callers that prefer a non-method form. Equivalent to
/// `registry.enroll(worker_id, tenant_id)`.
pub fn enroll_worker(
    registry: &WorkerLearningRegistry,
    worker_id: WorkerId,
    tenant_id: TenantId,
) -> bool {
    registry.enroll(worker_id, tenant_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionStore;

    async fn make_registry() -> WorkerLearningRegistry {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        let skills = Arc::new(SkillStore::new(Arc::clone(&store)));
        WorkerLearningRegistry::new(
            skills,
            PerWorkerLearningConfig::default(),
            Duration::from_secs(3600),
        )
    }

    #[tokio::test]
    async fn enroll_inserts_new_worker() {
        let registry = make_registry().await;
        let worker = WorkerId::new();
        let tenant = TenantId::new();
        assert!(registry.enroll(worker, tenant));
        assert!(registry.is_enrolled(worker));
        assert_eq!(registry.enrolled_count(), 1);
    }

    #[tokio::test]
    async fn enroll_is_idempotent() {
        let registry = make_registry().await;
        let worker = WorkerId::new();
        let tenant = TenantId::new();
        assert!(registry.enroll(worker, tenant));
        assert!(!registry.enroll(worker, tenant));
        assert_eq!(registry.enrolled_count(), 1);
    }

    #[tokio::test]
    async fn enroll_worker_free_function_matches_method() {
        let registry = make_registry().await;
        let worker = WorkerId::new();
        let tenant = TenantId::new();
        assert!(enroll_worker(&registry, worker, tenant));
        assert!(!enroll_worker(&registry, worker, tenant));
    }

    #[tokio::test]
    async fn drop_aborts_outstanding_handles() {
        let registry = make_registry().await;
        let worker = WorkerId::new();
        let tenant = TenantId::new();
        registry.enroll(worker, tenant);
        let stored = registry.handles.lock().unwrap();
        let handle_finished_before = stored.get(&worker).unwrap().is_finished();
        drop(stored);
        drop(registry);
        // We can't observe the aborted handle directly (it's moved on
        // drop), but the `is_finished` snapshot above proves the cron
        // was live before Drop and the abort path runs without panic.
        assert!(!handle_finished_before);
    }
}
