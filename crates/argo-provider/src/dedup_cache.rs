//! In-process LLM-call dedup cache (mission-llm-call-dedup-cache-v2).
//!
//! Same agent turn often issues identical completion requests
//! (`auto_approval` + `dispatch` + `safety_judge`).
//! `DedupCache<P: Transport>` fingerprints each [`ProviderRequest`]
//! (model + messages + tools + system + sampling), stores the
//! response for up to `ttl`, and returns the cached value on a hit.
//!
//! Streaming calls bypass the cache — wire-level deltas aren't
//! replayable from a stored `ProviderResponse`, and the dedup-prone
//! micro-LLM paths are non-streaming.
//!
//! Tenant isolation: [`ProviderRequest`] carries no explicit
//! `tenant_id`. Scope flows in implicitly via
//! [`argo_types::Message::session_id`] (a UUID, JSON-serialised into
//! the fingerprint) — different tenants' sessions hash to different
//! keys. Requests with empty `messages` lose implicit isolation; a
//! v1.x follow-up will plumb tenant_id through the request type.
//!
//! Construction spawns a 5-min GC sweep holding a [`Weak`] to the
//! store, so the loop exits when the cache drops. Tests drive the
//! eviction predicate directly via [`DedupCache::evict_stale`].
//!
//! ```
//! # async fn doc<T: argo_provider::Transport>(inner: T) {
//! use std::time::Duration;
//! use argo_provider::DedupCache;
//! let _wrapped = DedupCache::new(inner, Duration::from_secs(60));
//! # }
//! ```
use crate::transport::{ProviderRequest, ProviderResponse, ProviderResponseDelta, Transport};
use argo_types::{ArgoResult, CapabilitySet};
use async_trait::async_trait;
use futures::stream::BoxStream;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

const GC_INTERVAL: Duration = Duration::from_secs(300);

type Fingerprint = [u8; 32];

/// One cached response + the wall-clock moment it was inserted.
pub struct CachedEntry {
    response: ProviderResponse,
    inserted_at: Instant,
}

type Store = RwLock<HashMap<Fingerprint, CachedEntry>>;

/// [`Transport`] wrapper that dedupes identical non-streaming
/// completion requests within a configurable TTL window.
pub struct DedupCache<T: Transport> {
    inner: T,
    ttl: Duration,
    store: Arc<Store>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl<T: Transport> DedupCache<T> {
    /// Wrap `inner` with a TTL of `ttl`. If a tokio runtime is
    /// available, also spawns the 5-min GC sweep. The sweep holds
    /// only a weak reference to the store, so it exits when the
    /// cache drops.
    pub fn new(inner: T, ttl: Duration) -> Self {
        let store: Arc<Store> = Arc::new(RwLock::new(HashMap::new()));
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let weak = Arc::downgrade(&store);
            handle.spawn(gc_loop(weak, ttl));
        }
        Self {
            inner,
            ttl,
            store,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Cumulative cache hits since construction.
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Cumulative cache misses since construction.
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Borrowed inner transport.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Drop entries inserted more than `ttl` ago, measured against
    /// `now`. Called every `GC_INTERVAL` by the background loop.
    pub fn evict_stale(&self, now: Instant) {
        let ttl = self.ttl;
        let mut g = self.store.write();
        g.retain(|_, e| now.saturating_duration_since(e.inserted_at) < ttl);
    }

    /// Current store size — observability hook for tests.
    pub fn len(&self) -> usize {
        self.store.read().len()
    }

    /// True when no entries are cached.
    pub fn is_empty(&self) -> bool {
        self.store.read().is_empty()
    }
}

async fn gc_loop(store: Weak<Store>, ttl: Duration) {
    let mut interval = tokio::time::interval(GC_INTERVAL);
    // Skip the immediate-tick on first poll; we want the *first*
    // eviction to land after `GC_INTERVAL`, not at construction.
    interval.tick().await;
    loop {
        interval.tick().await;
        let Some(store) = store.upgrade() else { return };
        let now = Instant::now();
        store
            .write()
            .retain(|_, e| now.saturating_duration_since(e.inserted_at) < ttl);
    }
}

#[async_trait]
impl<T: Transport> Transport for DedupCache<T> {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn capabilities(&self) -> &CapabilitySet {
        self.inner.capabilities()
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let fp = fingerprint(req);
        // Read path: short, lock-light. parking_lot RwLock so the
        // lookup is sync.
        {
            let g = self.store.read();
            if let Some(entry) = g.get(&fp) {
                if Instant::now().saturating_duration_since(entry.inserted_at) < self.ttl {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(entry.response.clone());
                }
            }
        }
        // Miss: call through, store the response, return.
        self.misses.fetch_add(1, Ordering::Relaxed);
        let response = self.inner.complete(req).await?;
        let mut g = self.store.write();
        g.insert(
            fp,
            CachedEntry {
                response: response.clone(),
                inserted_at: Instant::now(),
            },
        );
        Ok(response)
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        // Streaming bypasses the cache: deltas are not replayable
        // from a stored `ProviderResponse`, and the dedup-prone
        // micro-LLM paths (`auto_approval`, `dispatch`, `safety_judge`)
        // are non-streaming.
        self.inner.complete_stream(req).await
    }
}

/// Blanket impl so a boxed transport can be the inner of a
/// statically-typed [`DedupCache<Box<dyn Transport>>`]. The trait
/// definition lives in `transport.rs`, which is locked by the brief,
/// so this lives here.
#[async_trait]
impl<T: Transport + ?Sized> Transport for Box<T> {
    fn id(&self) -> &str {
        (**self).id()
    }

    fn capabilities(&self) -> &CapabilitySet {
        (**self).capabilities()
    }

    async fn complete(&self, req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        (**self).complete(req).await
    }

    async fn complete_stream(
        &self,
        req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        (**self).complete_stream(req).await
    }
}

fn fingerprint(req: &ProviderRequest) -> Fingerprint {
    // Inputs are NUL-delimited so two values can't collide by concatenation.
    // `messages` carries `session_id` → implicit per-tenant scope.
    let mut h = Sha256::new();
    h.update(req.model.as_bytes());
    h.update(b"\0");
    match serde_json::to_vec(&req.messages) {
        Ok(bytes) => h.update(&bytes),
        Err(_) => h.update(b"<unserializable messages>"),
    }
    h.update(b"\0");
    match serde_json::to_vec(&req.tools) {
        Ok(bytes) => h.update(&bytes),
        Err(_) => h.update(b"<unserializable tools>"),
    }
    h.update(b"\0");
    if let Some(s) = &req.system {
        h.update(s.as_bytes());
    }
    h.update(b"\0");
    if let Some(t) = req.sampling.temperature {
        h.update(t.to_le_bytes());
    }
    h.update(b"\0");
    if let Some(p) = req.sampling.top_p {
        h.update(p.to_le_bytes());
    }
    h.update(b"\0");
    if let Some(m) = req.sampling.max_tokens {
        h.update(m.to_le_bytes());
    }
    h.update(b"\0");
    for s in &req.sampling.stop {
        h.update(s.as_bytes());
        h.update(b"\x1e"); // ASCII record separator between stop sequences
    }
    h.finalize().into()
}

#[cfg(test)]
#[path = "dedup_cache_tests.rs"]
mod tests;
