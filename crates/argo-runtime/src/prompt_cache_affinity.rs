//! Sticky-session affinity for the Anthropic prompt cache
//! (`mission-prompt-cache-session-affinity`).
//!
//! Anthropic's `cache_control` markers on the system prompt only
//! produce a cache **read** when the same `session_id` replays
//! inside the 5-minute TTL window. Today every turn picks a fresh
//! provider session id, so each cache row is created and
//! immediately abandoned — the read counter never climbs.
//!
//! This module is a tiny in-memory registry that maps
//! `(tenant_id, worker_id)` → most-recent `SessionId` together
//! with the `Instant` it was last recorded. Callers ask
//! [`PromptCacheAffinity::acquire_or_mint`] at turn-start; if the
//! recorded id is still within `ttl_secs` they get it back, else
//! the registry mints a fresh `SessionId` and remembers it for
//! the next call. `record` lets callers override the recorded id
//! when the turn actually completed against a different session.
//!
//! Per-tenant isolation is the only correctness invariant:
//! `(tenant_a, worker_x)` MUST NOT see `(tenant_b, worker_x)`'s
//! session id at any wall-clock. Tests cover this case
//! explicitly.
//!
//! Disabled-config path: when affinity is off, every call to
//! [`PromptCacheAffinity::acquire_or_mint`] returns a fresh
//! `SessionId` without touching the map — the pre-brief behavior.
//!
//! No new workspace dependencies. `std::sync::Mutex` +
//! `std::time::Instant` are sufficient for v1; if profiling
//! eventually shows contention we swap to `DashMap` in a
//! follow-up.

use argo_types::{SessionId, TenantId};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Affinity registry — keeps the most-recent `SessionId` for each
/// `(tenant_id, worker_id)` pair within a TTL window.
///
/// Cheap to clone the surrounding `Arc<PromptCacheAffinity>`;
/// the inner state is behind a single `Mutex<HashMap>`.
///
/// ```
/// use argo_runtime::prompt_cache_affinity::PromptCacheAffinity;
/// use argo_types::TenantId;
///
/// let affinity = PromptCacheAffinity::enabled(std::time::Duration::from_secs(270));
/// let tenant = TenantId::new();
/// let first = affinity.acquire_or_mint(tenant, "worker-1");
/// let second = affinity.acquire_or_mint(tenant, "worker-1");
/// assert_eq!(first, second, "same pair within TTL reuses the id");
/// ```
#[derive(Debug)]
pub struct PromptCacheAffinity {
    enabled: bool,
    ttl: Duration,
    state: Mutex<HashMap<AffinityKey, RecordedSession>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AffinityKey {
    tenant: TenantId,
    worker: String,
}

#[derive(Debug, Clone, Copy)]
struct RecordedSession {
    session: SessionId,
    recorded_at: Instant,
}

impl PromptCacheAffinity {
    /// Build an enabled registry with the given TTL window.
    /// Operators that want strict cache-affinity-off pass through
    /// [`Self::disabled`] instead.
    #[must_use]
    pub fn enabled(ttl: Duration) -> Self {
        Self {
            enabled: true,
            ttl,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Build a disabled registry. Every `acquire_or_mint` mints a
    /// fresh `SessionId` and `record` is a no-op — restores the
    /// pre-brief per-turn-fresh-id behavior so operators can
    /// opt out of affinity without conditional wiring at the call
    /// site.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ttl: Duration::from_secs(0),
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Whether this registry is in enabled mode. Mostly useful
    /// for tests and CLI introspection — the call sites should
    /// stay un-branched.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// TTL window the registry was built with.
    #[must_use]
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Look up the recorded session id for
    /// `(tenant, worker_id)`; if one exists and is still within
    /// the TTL window, return it. Otherwise mint a fresh
    /// `SessionId`, remember it, and return that.
    ///
    /// When the registry is disabled the call always returns a
    /// fresh `SessionId` without touching internal state.
    pub fn acquire_or_mint(&self, tenant: TenantId, worker_id: &str) -> SessionId {
        if !self.enabled {
            return SessionId::new();
        }
        let now = Instant::now();
        let key = AffinityKey {
            tenant,
            worker: worker_id.to_string(),
        };
        let mut map = match self.state.lock() {
            Ok(g) => g,
            // Poisoned mutex means a prior caller panicked while
            // holding the lock. The map content is still well-
            // formed (no torn writes — `HashMap::insert` is
            // panic-free for our key/value types), so recover and
            // continue. The alternative — propagating the panic
            // — would take down every subsequent turn.
            Err(poison) => poison.into_inner(),
        };
        if let Some(existing) = map.get(&key) {
            if now.saturating_duration_since(existing.recorded_at) <= self.ttl {
                return existing.session;
            }
        }
        let fresh = SessionId::new();
        map.insert(
            key,
            RecordedSession {
                session: fresh,
                recorded_at: now,
            },
        );
        fresh
    }

    /// Refresh the recorded session id for `(tenant, worker_id)`.
    /// Call this on turn-end with the session that actually ran,
    /// so the next turn-start sees its `recorded_at` advanced and
    /// the TTL window slides forward.
    ///
    /// When the registry is disabled this is a no-op.
    pub fn record(&self, tenant: TenantId, worker_id: &str, session: SessionId) {
        if !self.enabled {
            return;
        }
        let key = AffinityKey {
            tenant,
            worker: worker_id.to_string(),
        };
        let mut map = match self.state.lock() {
            Ok(g) => g,
            Err(poison) => poison.into_inner(),
        };
        map.insert(
            key,
            RecordedSession {
                session,
                recorded_at: Instant::now(),
            },
        );
    }

    /// Drop the recorded session id for `(tenant, worker_id)`.
    /// Useful when the orchestrator knows the worker's logical
    /// session has ended (e.g. tenant suspension, worker reset)
    /// and a stale cache hit would be incorrect.
    pub fn forget(&self, tenant: TenantId, worker_id: &str) {
        if !self.enabled {
            return;
        }
        let key = AffinityKey {
            tenant,
            worker: worker_id.to_string(),
        };
        let mut map = match self.state.lock() {
            Ok(g) => g,
            Err(poison) => poison.into_inner(),
        };
        map.remove(&key);
    }

    /// Count of currently-recorded affinity entries. For tests
    /// and operator introspection — not a hot path.
    #[must_use]
    pub fn len(&self) -> usize {
        match self.state.lock() {
            Ok(g) => g.len(),
            Err(poison) => poison.into_inner().len(),
        }
    }

    /// Whether the registry currently holds any recorded
    /// affinities.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PromptCacheAffinity {
    /// 270s = Anthropic's 5-minute cache TTL minus 30s headroom
    /// for clock skew and network jitter. Operators that run on
    /// dedicated infra can raise this via runtime config.
    fn default() -> Self {
        Self::enabled(Duration::from_secs(270))
    }
}
