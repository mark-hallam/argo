//! Typed scaffold for blob storage — `AttachedBlob`, `BlobRef`,
//! `BlobStore` trait, and an in-process `MemoryAttachedBlob`
//! implementation used by tests.
//!
//! No persistence yet: this brief lands the types only so downstream
//! work (`claude-cu-screenshot-storage`) can plug in the SQLite-backed
//! `BlobStore` impl without churn.

use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

/// Opaque identifier for a stored blob. UUIDv7 so ids sort
/// lexicographically by creation time.
///
/// ```
/// use argo_store::attached_blob::BlobId;
/// let id = BlobId::new();
/// let s = id.to_string();
/// assert_eq!(s.len(), 36);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobId(pub Uuid);

impl BlobId {
    /// Generate a fresh UUIDv7 blob identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for BlobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BlobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// A blob payload plus the metadata needed to round-trip it through a
/// [`BlobStore`].
///
/// `sha256` is computed by [`AttachedBlob::new`] from `bytes`; if the
/// caller fabricates the struct directly, they own the invariant that
/// `sha256` matches `bytes`.
///
/// ```
/// use argo_store::attached_blob::AttachedBlob;
/// let blob = AttachedBlob::new(b"hello".to_vec(), "text/plain".into());
/// assert_eq!(blob.size_bytes, 5);
/// assert_eq!(blob.bytes, b"hello");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedBlob {
    /// Stable identifier assigned at construction time.
    pub id: BlobId,
    /// Raw payload bytes.
    pub bytes: Vec<u8>,
    /// IANA MIME type (`image/png`, `application/octet-stream`, ...).
    pub mime: String,
    /// Length of `bytes` cached on the struct so callers can read it
    /// without re-measuring the buffer.
    pub size_bytes: u64,
    /// SHA-256 of `bytes`, used for content addressing and
    /// integrity checks.
    pub sha256: [u8; 32],
    /// Wall-clock instant the blob was constructed.
    pub created_at: DateTime<Utc>,
}

impl AttachedBlob {
    /// Build an `AttachedBlob` from raw bytes; computes `size_bytes`
    /// and `sha256` and stamps `created_at` to `Utc::now()`.
    ///
    /// ```
    /// use argo_store::attached_blob::AttachedBlob;
    /// let a = AttachedBlob::new(vec![1, 2, 3], "application/octet-stream".into());
    /// let b = AttachedBlob::new(vec![1, 2, 3], "application/octet-stream".into());
    /// assert_eq!(a.sha256, b.sha256);
    /// assert_ne!(a.id, b.id);
    /// ```
    #[must_use]
    pub fn new(bytes: Vec<u8>, mime: String) -> Self {
        let size_bytes = bytes.len() as u64;
        let sha256 = sha256_of(&bytes);
        Self {
            id: BlobId::new(),
            bytes,
            mime,
            size_bytes,
            sha256,
            created_at: Utc::now(),
        }
    }

    /// Project this blob into a lightweight [`BlobRef`] (no payload).
    #[must_use]
    pub fn as_ref(&self) -> BlobRef {
        BlobRef {
            id: self.id,
            mime: self.mime.clone(),
            size_bytes: self.size_bytes,
            sha256: self.sha256,
        }
    }
}

/// Lightweight pointer to a stored blob — the metadata needed to
/// fetch or display a blob without dragging the full payload around.
///
/// ```
/// use argo_store::attached_blob::{AttachedBlob, BlobRef};
/// let blob = AttachedBlob::new(b"x".to_vec(), "text/plain".into());
/// let r: BlobRef = blob.as_ref();
/// assert_eq!(r.size_bytes, 1);
/// assert_eq!(r.id, blob.id);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRef {
    /// Identifier of the referenced blob.
    pub id: BlobId,
    /// MIME type of the referenced blob.
    pub mime: String,
    /// Length in bytes of the referenced blob.
    pub size_bytes: u64,
    /// SHA-256 of the referenced blob.
    pub sha256: [u8; 32],
}

/// Async storage interface for `AttachedBlob` payloads.
///
/// The trait is async + `Send + Sync` so SQLite, S3, and in-memory
/// implementations all share the same surface. `get` returns
/// [`ArgoError::NotFound`] when the requested id is unknown.
///
/// ```
/// # use argo_store::attached_blob::{AttachedBlob, BlobStore, MemoryAttachedBlob};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let store = MemoryAttachedBlob::default();
/// let blob = AttachedBlob::new(b"hi".to_vec(), "text/plain".into());
/// let r = store.put(blob.clone()).await?;
/// let fetched = store.get(r.id).await?;
/// assert_eq!(fetched.bytes, blob.bytes);
/// # Ok(()) }
/// ```
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Persist `blob` and return the lightweight reference.
    async fn put(&self, blob: AttachedBlob) -> ArgoResult<BlobRef>;
    /// Fetch the full blob by id, or [`ArgoError::NotFound`].
    async fn get(&self, id: BlobId) -> ArgoResult<AttachedBlob>;
}

/// In-process [`BlobStore`] backed by a `HashMap`. Intended for tests
/// and downstream development before the SQLite-backed implementation
/// lands.
///
/// ```
/// use argo_store::attached_blob::MemoryAttachedBlob;
/// let store = MemoryAttachedBlob::default();
/// assert_eq!(store.len(), 0);
/// ```
#[derive(Debug, Default)]
pub struct MemoryAttachedBlob {
    inner: Mutex<HashMap<BlobId, AttachedBlob>>,
}

impl MemoryAttachedBlob {
    /// Construct an empty in-memory store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of blobs currently held. Test helper.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// Whether the store is empty. Test helper.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl BlobStore for MemoryAttachedBlob {
    async fn put(&self, blob: AttachedBlob) -> ArgoResult<BlobRef> {
        let r = blob.as_ref();
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| ArgoError::Provider(format!("memory blob store poisoned: {e}")))?;
        guard.insert(blob.id, blob);
        Ok(r)
    }

    async fn get(&self, id: BlobId) -> ArgoResult<AttachedBlob> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| ArgoError::Provider(format!("memory blob store poisoned: {e}")))?;
        guard
            .get(&id)
            .cloned()
            .ok_or_else(|| ArgoError::NotFound(format!("blob {id}")))
    }
}

fn sha256_of(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

#[cfg(test)]
#[path = "attached_blob_tests.rs"]
mod tests;
