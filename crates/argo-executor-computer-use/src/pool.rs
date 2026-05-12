//! VM pool keyed by `(tenant, persona, profile)` with a 10-min
//! idle TTL. The pool sits between the executor and the E2B
//! client: when the executor needs a sandbox it asks the pool;
//! the pool either returns an idle sandbox (resetting its idle
//! timer) or provisions a fresh one. When a sandbox sits idle
//! past its TTL the pool's reaper closes it.
//!
//! v1 keeps the pool in-process — there's no Redis or external
//! coordination. Multi-process coordination lands when we run
//! more than one worker daemon per tenant; until then the
//! single-binary invariant means all dispatches share this map.

use crate::e2b::{E2bClient, Sandbox as E2bSandbox};
use crate::sandbox_prewarm::{PrewarmedSandbox, SandboxPrewarmer};
use crate::CuError;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Default idle TTL: 10 minutes per the brief.
pub const DEFAULT_IDLE_TTL: Duration = Duration::from_secs(600);

/// Pool key. The brief specifies `(tenant, persona, profile)`.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PoolKey {
    pub tenant_id: String,
    pub persona_id: String,
    pub profile: String,
}

#[derive(Debug)]
struct Slot {
    sandbox: E2bSandbox,
    last_used: Instant,
}

/// Sandbox pool. Cloneable — `Arc<Mutex<…>>` underneath so the
/// executor can hand clones to async tasks.
#[derive(Debug, Clone)]
pub struct SandboxPool {
    inner: Arc<Mutex<PoolInner>>,
    e2b: E2bClient,
    idle_ttl: Duration,
    /// Pre-warm reservoir. `None` until the operator opts in via
    /// `[runtime.sandbox_prewarm] enabled = true` and the executor
    /// stamps one on via [`Self::with_prewarmer`].
    prewarmer: Option<Arc<Mutex<SandboxPrewarmer>>>,
}

#[derive(Debug, Default)]
struct PoolInner {
    slots: HashMap<PoolKey, Slot>,
}

impl SandboxPool {
    /// Build a pool that uses `client` to provision new
    /// sandboxes on demand.
    #[must_use]
    pub fn new(e2b: E2bClient) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PoolInner::default())),
            e2b,
            idle_ttl: DEFAULT_IDLE_TTL,
            prewarmer: None,
        }
    }

    /// Override the idle TTL. Useful for tests that want to
    /// observe eviction without waiting 10 minutes.
    #[must_use]
    pub fn with_idle_ttl(mut self, ttl: Duration) -> Self {
        self.idle_ttl = ttl;
        self
    }

    /// Attach a [`SandboxPrewarmer`]. Without one, [`Self::acquire`]
    /// behaves exactly as before (cold-open on every slot miss).
    /// With one, the pool consults the reservoir for matching
    /// templates before falling through to E2B, and fires
    /// background pre-warm bursts when the spawn rate crosses the
    /// configured threshold.
    #[must_use]
    pub fn with_prewarmer(mut self, prewarmer: Arc<Mutex<SandboxPrewarmer>>) -> Self {
        self.prewarmer = Some(prewarmer);
        self
    }

    /// Acquire a sandbox for the given key + sandbox template.
    /// Returns an idle sandbox if available, otherwise consults
    /// the pre-warm reservoir (when configured), otherwise opens
    /// a fresh one via E2B. The pool stamps the slot's
    /// `last_used` so the reaper doesn't clobber it mid-use.
    pub async fn acquire(
        &self,
        key: PoolKey,
        sandbox_template: &str,
    ) -> Result<E2bSandbox, CuError> {
        // First pass: hit the pool while holding the lock.
        {
            let mut inner = self.inner.lock().await;
            if let Some(slot) = inner.slots.get_mut(&key) {
                slot.last_used = Instant::now();
                return Ok(slot.sandbox.clone());
            }
        }
        // Slot miss: record demand + check the pre-warm reservoir
        // (audit row + slot-insert happens inside the helper).
        if let Some(sandbox) = self.try_take_prewarmed(&key, sandbox_template).await {
            self.maybe_trigger_prewarm(sandbox_template).await;
            return Ok(sandbox);
        }
        // Reservoir miss: provision a fresh sandbox without
        // holding the lock. Re-acquire to insert.
        let fresh = self.e2b.open_sandbox(sandbox_template).await?;
        let mut inner = self.inner.lock().await;
        // Race-safe: another caller may have inserted while we
        // were provisioning. If so, prefer the existing slot
        // and close the duplicate.
        if let Some(slot) = inner.slots.get_mut(&key) {
            slot.last_used = Instant::now();
            let existing = slot.sandbox.clone();
            drop(inner);
            // Best-effort cleanup; ignore errors so the pool
            // stays usable even if the redundant sandbox
            // refuses to close.
            let _ = self.e2b.close_sandbox(&fresh).await;
            return Ok(existing);
        }
        inner.slots.insert(
            key,
            Slot {
                sandbox: fresh.clone(),
                last_used: Instant::now(),
            },
        );
        drop(inner);
        // Cold spawn complete; the spawn was already recorded by
        // `try_take_prewarmed`, so the burst trigger can fire now.
        self.maybe_trigger_prewarm(sandbox_template).await;
        Ok(fresh)
    }

    /// Record a spawn demand event and try to satisfy it from
    /// the pre-warm reservoir. Returns `Some(sandbox)` on hit
    /// (slot updated, audit emitted) or `None` if pre-warming is
    /// off or no entry matches the template. Always records the
    /// demand event when a prewarmer is attached, so the
    /// rate-trigger sees the load even when the reservoir is
    /// empty.
    async fn try_take_prewarmed(
        &self,
        key: &PoolKey,
        sandbox_template: &str,
    ) -> Option<E2bSandbox> {
        let prewarmer = self.prewarmer.as_ref()?;
        let consumed = {
            let mut p = prewarmer.lock().await;
            p.record_spawn(Instant::now());
            p.try_consume(sandbox_template)
        };
        let sandbox = consumed?;
        let mut inner = self.inner.lock().await;
        inner.slots.insert(
            key.clone(),
            Slot {
                sandbox: sandbox.clone(),
                last_used: Instant::now(),
            },
        );
        Some(sandbox)
    }

    /// Consult the prewarmer's burst + hourly triggers and, if
    /// either fires, spawn a background task to open the
    /// requested number of sandboxes and admit them. The trigger
    /// math is synchronous + mutex-bounded; the actual E2B I/O
    /// runs detached so the dispatch path is never blocked on
    /// pre-warm. Best-effort: a failed `open_sandbox` is logged
    /// and otherwise ignored — pre-warming is an optimisation,
    /// not a correctness requirement.
    async fn maybe_trigger_prewarm(&self, sandbox_template: &str) {
        let Some(prewarmer) = self.prewarmer.as_ref() else {
            return;
        };
        let now = Instant::now();
        let now_utc = chrono::Utc::now();
        let to_open = {
            let mut p = prewarmer.lock().await;
            let burst = p.should_burst_prewarm(now);
            let hourly = u32::from(p.should_hourly_prewarm(now_utc));
            burst.saturating_add(hourly)
        };
        if to_open == 0 {
            return;
        }
        let prewarmer = Arc::clone(prewarmer);
        let client = self.e2b.clone();
        let template = sandbox_template.to_owned();
        tokio::spawn(async move {
            for _ in 0..to_open {
                match client.open_sandbox(&template).await {
                    Ok(sandbox) => {
                        let entry = PrewarmedSandbox {
                            template: template.clone(),
                            sandbox: sandbox.clone(),
                            prewarmed_at: Instant::now(),
                        };
                        let admitted = {
                            let mut p = prewarmer.lock().await;
                            p.try_admit(entry)
                        };
                        if !admitted {
                            // Reservoir full between trigger and
                            // admit — close so we don't leak.
                            let _ = client.close_sandbox(&sandbox).await;
                        }
                    }
                    Err(err) => {
                        tracing::warn!(
                            kind = "runtime.sandbox_prewarm_open_failed",
                            template = %template,
                            error = %err,
                            "pre-warm open_sandbox failed"
                        );
                    }
                }
            }
        });
    }

    /// Mark a sandbox as idle (resets the TTL clock). Called by
    /// the executor at end-of-dispatch — the next caller for the
    /// same key reuses it, the reaper closes it after the TTL.
    pub async fn release(&self, key: &PoolKey) {
        let mut inner = self.inner.lock().await;
        if let Some(slot) = inner.slots.get_mut(key) {
            slot.last_used = Instant::now();
        }
    }

    /// Run one reaper pass. Closes every sandbox whose
    /// `last_used` is older than the TTL. Returns the count
    /// closed so callers can log a breadcrumb.
    pub async fn reap_idle(&self) -> Result<usize, CuError> {
        let now = Instant::now();
        let evicted: Vec<(PoolKey, E2bSandbox)> = {
            let mut inner = self.inner.lock().await;
            let mut to_evict = Vec::new();
            inner.slots.retain(|k, slot| {
                let idle = now.duration_since(slot.last_used);
                if idle > self.idle_ttl {
                    to_evict.push((k.clone(), slot.sandbox.clone()));
                    false
                } else {
                    true
                }
            });
            to_evict
        };
        let count = evicted.len();
        for (_key, sandbox) in evicted {
            // Best-effort close; don't fail the reaper for one
            // stuck sandbox.
            let _ = self.e2b.close_sandbox(&sandbox).await;
        }
        Ok(count)
    }

    /// Snapshot of pool occupancy. Used by tests + the
    /// `argo doctor` integration that lands later.
    pub async fn occupancy(&self) -> usize {
        let inner = self.inner.lock().await;
        inner.slots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(profile: &str) -> PoolKey {
        PoolKey {
            tenant_id: "tenant-A".into(),
            persona_id: "maya".into(),
            profile: profile.into(),
        }
    }

    #[tokio::test]
    async fn occupancy_starts_empty() {
        // We can't easily `acquire` without a live E2B endpoint,
        // but the structural surface is testable.
        let e2b = E2bClient::new("http://localhost:1".into(), "k".into(), 1)
            .expect("client builds");
        let pool = SandboxPool::new(e2b);
        assert_eq!(pool.occupancy().await, 0);
    }

    #[tokio::test]
    async fn release_on_unknown_key_is_noop() {
        let e2b = E2bClient::new("http://localhost:1".into(), "k".into(), 1)
            .expect("client builds");
        let pool = SandboxPool::new(e2b);
        pool.release(&key("marketplace_browser")).await;
        assert_eq!(pool.occupancy().await, 0);
    }

    #[tokio::test]
    async fn reap_idle_returns_zero_on_empty_pool() {
        let e2b = E2bClient::new("http://localhost:1".into(), "k".into(), 1)
            .expect("client builds");
        let pool = SandboxPool::new(e2b).with_idle_ttl(Duration::from_millis(1));
        assert_eq!(pool.reap_idle().await.unwrap(), 0);
    }

    #[test]
    fn pool_key_equality_is_field_wise() {
        let k1 = key("marketplace_browser");
        let k2 = key("marketplace_browser");
        assert_eq!(k1, k2);
        let mut k3 = key("marketplace_browser");
        k3.persona_id = "different".into();
        assert_ne!(k1, k3);
    }
}
