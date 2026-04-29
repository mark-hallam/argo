//! TTL cache for `(session_id, hash(user_message)) → tier` decisions
//! (§24.3 — 60 s window so retries don't re-classify).
use argo_types::SessionId;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default cache TTL per spec.
pub const DEFAULT_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
struct Key {
    session_id: SessionId,
    msg_hash: [u8; 32],
}

#[derive(Debug)]
struct Entry {
    tier: String,
    inserted_at: Instant,
}

/// In-process cache. Lazy expiration on lookup keeps the implementation
/// allocation-free in the steady state.
#[derive(Debug)]
pub struct TierCache {
    inner: Mutex<HashMap<Key, Entry>>,
    ttl: Duration,
}

impl Default for TierCache {
    fn default() -> Self {
        Self::with_ttl(DEFAULT_TTL)
    }
}

impl TierCache {
    /// New cache with the given TTL.
    #[must_use]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// SHA-256 the message body (§24.3).
    #[must_use]
    pub fn hash_message(message: &str) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(message.as_bytes());
        let out = h.finalize();
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&out);
        buf
    }

    /// Look up a cached tier. Lazy-expires entries past `ttl`.
    pub fn get(&self, session_id: SessionId, msg_hash: [u8; 32]) -> Option<String> {
        let mut g = self.inner.lock();
        let key = Key {
            session_id,
            msg_hash,
        };
        let now = Instant::now();
        let stale = g
            .get(&key)
            .map(|e| now.duration_since(e.inserted_at) > self.ttl)
            .unwrap_or(false);
        if stale {
            g.remove(&key);
            return None;
        }
        g.get(&key).map(|e| e.tier.clone())
    }

    /// Insert or refresh a cached tier.
    pub fn insert(&self, session_id: SessionId, msg_hash: [u8; 32], tier: String) {
        let key = Key {
            session_id,
            msg_hash,
        };
        let entry = Entry {
            tier,
            inserted_at: Instant::now(),
        };
        self.inner.lock().insert(key, entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miss_then_hit() {
        let c = TierCache::default();
        let s = SessionId::new();
        let h = TierCache::hash_message("hello");
        assert!(c.get(s, h).is_none());
        c.insert(s, h, "simple".into());
        assert_eq!(c.get(s, h).as_deref(), Some("simple"));
    }

    #[test]
    fn different_session_misses() {
        let c = TierCache::default();
        let h = TierCache::hash_message("hi");
        c.insert(SessionId::new(), h, "complex".into());
        assert!(c.get(SessionId::new(), h).is_none());
    }

    #[test]
    fn different_message_misses() {
        let c = TierCache::default();
        let s = SessionId::new();
        c.insert(s, TierCache::hash_message("hi"), "complex".into());
        assert!(c.get(s, TierCache::hash_message("hello")).is_none());
    }

    #[test]
    fn stale_entries_expire() {
        let c = TierCache::with_ttl(Duration::from_millis(0));
        let s = SessionId::new();
        let h = TierCache::hash_message("hi");
        c.insert(s, h, "simple".into());
        std::thread::sleep(Duration::from_millis(2));
        assert!(c.get(s, h).is_none());
    }

    #[test]
    fn hash_is_stable_across_calls() {
        let a = TierCache::hash_message("hello");
        let b = TierCache::hash_message("hello");
        assert_eq!(a, b);
        let c = TierCache::hash_message("world");
        assert_ne!(a, c);
    }
}
