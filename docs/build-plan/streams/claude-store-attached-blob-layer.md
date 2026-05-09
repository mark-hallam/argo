# claude-store-attached-blob-layer — define AttachedBlob + BlobRef + NotFound error variant

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep.**

**Authored against**: origin/main c2057ac1 on 2026-05-09 (conductor-authored prereq for claude-cu-screenshot-storage which in turn unblocks claude-cu-action-replay-cli).

## Why

`claude-cu-screenshot-storage` and downstream `claude-cu-action-replay-cli` both need a typed blob-storage layer for screenshots and other action-replay artefacts. The current `argo-store` doesn't have one. This brief lands the typed scaffold (NEW types + a NEW `ArgoError::NotFound` variant) without any of the storage logic.

## Approach (concrete)

1. **NEW** `crates/argo-store/src/attached_blob.rs` (~150 LOC):
   - `pub struct AttachedBlob { id: BlobId, bytes: Vec<u8>, mime: String, size_bytes: u64, sha256: [u8; 32], created_at: DateTime<Utc> }`
   - `pub struct BlobRef { id: BlobId, mime: String, size_bytes: u64, sha256: [u8; 32] }` — lightweight pointer (no bytes)
   - `pub struct MemoryAttachedBlob { ... }` — in-process variant for testing
   - `pub trait BlobStore { async fn put(&self, blob: AttachedBlob) -> ArgoResult<BlobRef>; async fn get(&self, id: BlobId) -> ArgoResult<AttachedBlob>; }`
   - **No persistence yet** — only the types + trait.
2. **Add `NotFound` variant** to `ArgoError` in `crates/argo-types/src/lib.rs` (or wherever `ArgoError` lives — verify):
   - `NotFound(String)` — addressed entity does not exist. Maps to HTTP 404.
3. **NEW** `crates/argo-store/src/attached_blob_tests.rs` (~80 LOC) — basic round-trip test using `MemoryAttachedBlob`.

## Files you may touch

- `crates/argo-store/src/attached_blob.rs` (NEW, ≤ 200 LOC)
- `crates/argo-store/src/attached_blob_tests.rs` (NEW, ≤ 100 LOC)
- `crates/argo-store/src/lib.rs` — narrowly: pub mod
- `crates/argo-types/src/lib.rs` (or wherever `ArgoError` is defined) — narrowly: add `NotFound` variant

## Files you must NOT touch

- Any migration — this brief lands types only, no schema yet
- `argo-audit`, `argo-skill`, etc. — out of scope

## Acceptance criteria

- [ ] `AttachedBlob`, `BlobRef`, `MemoryAttachedBlob`, `BlobStore` trait all compile and have rustdoc with at least one example each
- [ ] `ArgoError::NotFound(String)` variant added; `Display` impl maps to a clear 404-shaped message
- [ ] `MemoryAttachedBlob` round-trip test passes (put then get returns same bytes)
- [ ] `cargo clippy -p argo-store -p argo-types --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-types` clean
- [ ] Every modified file ≤ 500 LOC

## Branch + PR

- Branch: `claude-store-attached-blob-layer`
- PR title: `store: AttachedBlob + BlobRef + NotFound error variant (claude-store-attached-blob-layer)`

## Unblocks

- `claude-cu-screenshot-storage` — needs `BlobStore` trait + `NotFound` to ship the SQLite-backed implementation.
- `claude-cu-action-replay-cli` (chained) — needs screenshot-storage to ship.
