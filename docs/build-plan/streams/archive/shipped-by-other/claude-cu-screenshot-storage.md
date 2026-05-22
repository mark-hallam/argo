# claude-cu-screenshot-storage — persist CU screenshots to argo-store-attached blob storage with retention

**Owner**: claude. **Depends on**: `argo-executor-computer-use` (#787) merged; `argo-store-attached` blob layer present.

## Why

Today CU executors write screenshots to `.skycot/outputs/<job_id>/actions/<action_id>/*.png` on disk. That works for single-host dev but has three gaps in production:

1. Screenshots disappear when the worker box is recycled — replay (cf. `claude-cu-action-replay-cli`) breaks
2. No retention story — disk fills if a tenant runs CU continuously
3. No tenant isolation — screenshots from tenant A live next to tenant B's on the same volume

This brief routes screenshots through `argo-store-attached` (the existing blob abstraction) with per-tenant prefixing and a TTL-driven retention sweep.

## Scope

1. **NEW** `crates/argo-cli/src/handlers/cu_screenshots.rs` (~280 LOC):
   - `CuScreenshotStore` struct wrapping `Arc<dyn AttachedBlob>` (existing trait)
   - `put(tenant_id, session_id, action_id, step_n, png_bytes) -> ArgoResult<BlobRef>` — writes under `cu/<tenant>/<session>/<action>/step_<n>.png`
   - `get(blob_ref) -> ArgoResult<Vec<u8>>` — fetch for replay
   - `list_session(tenant_id, session_id) -> ArgoResult<Vec<BlobRef>>` — enumerate for replay rendering
   - `sweep_expired(now: SystemTime, retention: Duration) -> ArgoResult<usize>` — delete shots older than retention; returns deleted count
2. **Schema migration** — NEW `crates/argo-store/migrations/<n>_cu_screenshot_index.sql`:
   - Table `cu_screenshot { id, tenant_id, session_id, action_id, step_n, blob_ref, captured_at, retention_until }`
   - Indexes on `(tenant_id, session_id)` + `(retention_until)` for sweep
3. **Wire CU executor** — narrowly modify `argo-executor-computer-use/src/executor.rs` to call `CuScreenshotStore::put` after each step, store returned `BlobRef` in the trajectory log
4. **Background sweep task** — spawn from gateway boot (existing pattern; see `mission-otlp-exporter` shape) running every 1h, deleting expired blobs + index rows
5. **Retention policy config** — read `cu.screenshot_retention_days` from `argo.toml` (default: 14d) via existing config plumbing

## Files you may touch

- `crates/argo-cli/src/handlers/cu_screenshots.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/cu_screenshots_tests.rs` (NEW, ≤ 280 LOC sibling) — 7 unit tests:
  - put → blob_ref round-trips through get
  - list_session returns ordered shots
  - sweep_expired deletes only old rows
  - sweep_expired is tenant-scoped
  - retention boundary (exactly at boundary)
  - missing blob → clean Provider error on get
  - tenant isolation (A cannot see B)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-store/migrations/<next>_cu_screenshot_index.sql` (NEW)
- `crates/argo-executor-computer-use/src/executor.rs` — narrowly: 1 call site per step, ~15 LOC delta
- `crates/argo-executor-computer-use/Cargo.toml` — narrowly: add `argo-store-attached` workspace dep
- `crates/argo-cli/src/gateway/boot.rs` (or equivalent) — narrowly: spawn the sweep task
- `crates/argo-cli/tests/cu_screenshot_storage_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration test using `MemoryAttachedBlob`

## Files you must NOT touch

- `argo-store-attached` trait — consume; don't extend
- Other migrations
- `Executor` trait
- `argo-channels` / `argo-mcp`

## Acceptance criteria

- [ ] CU executor writes each screenshot through `CuScreenshotStore::put` and the trajectory carries `blob_ref` instead of a local path
- [ ] Replay can fetch any historical screenshot via `get(blob_ref)`
- [ ] Sweep task deletes expired blobs + index rows every 1h; logs the delete count
- [ ] Retention configurable via `cu.screenshot_retention_days` in `argo.toml` (default 14d)
- [ ] Tenant isolation enforced (cross-tenant get returns `ArgoError::NotFound`)
- [ ] 7 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli -p argo-executor-computer-use` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-cu-screenshot-storage`
- PR title: `cu: persist screenshots to argo-store-attached with retention sweep (claude-cu-screenshot-storage)`

## Notes for the implementer

- Use `MemoryAttachedBlob` (existing test fixture) for unit + integration tests so they stay hermetic.
- `BlobRef` is the existing opaque handle from `argo-store-attached` — do not invent a new wrapper.
- The sweep task should respect the existing graceful-shutdown signal pattern; do not spawn a detached task.
- If the existing executor.rs is at-or-near 500 LOC, extract the screenshot put call into a small helper in `argo-executor-computer-use/src/screenshot_persist.rs` rather than bloating executor.rs.
