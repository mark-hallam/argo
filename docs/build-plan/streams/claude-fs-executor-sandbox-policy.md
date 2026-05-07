# claude-fs-executor-sandbox-policy — tighten FilesystemExecutor sandbox; deny outside `worker.fs_root`; audit every read/write

**Owner**: claude. **Depends on**: `mission-wire-mcp-action-executor` (#933) shape — same dispatch-side conventions. The `Filesystem` ExecutorKind variant exists today as a stub at `worker_actions_executor.rs:41`.

## Why

The `Filesystem` executor variant is currently a stub (returns `Err(ArgoError::Provider("executor 'filesystem' is not yet implemented"))`). When it lands as a real impl, the simplest version walks `payload.path` straight to `tokio::fs::read` — which lets a manifest declaration with `path = "/etc/passwd"` exfiltrate host secrets. That's a non-starter for the multi-tenant story.

This brief ships a **sandbox policy** layer for the FilesystemExecutor: every read/write is checked against `worker.fs_root` (configured per-worker), every operation is audit-logged with `{tenant, worker, op, path, decision}`, symlink escape is blocked via canonical-path resolution.

This is groundwork for the eventual real FilesystemExecutor impl; ship the policy first so the real impl plugs in safely.

## Scope

1. **NEW or extend** `crates/argo-cli/src/handlers/worker_actions_executor_fs_policy.rs` (~340 LOC):
   - `FsSandboxPolicy` struct: `fs_root: PathBuf`, `max_file_bytes: u64`, `read_allowlist: Vec<Glob>`, `write_allowlist: Vec<Glob>`
   - `check_read(path: &Path) -> ArgoResult<PathBuf>` — canonicalize + verify under `fs_root` + glob allowlist + emit audit event; returns canonical safe path
   - `check_write(path: &Path) -> ArgoResult<PathBuf>` — same as read, plus per-write allowlist
   - `check_size(bytes: u64) -> ArgoResult<()>` — bounds against `max_file_bytes`
   - All `check_*` methods log via `tracing` at INFO with structured fields `{tenant, worker, op, path, decision, reason?}`
2. **Wire into the (stub) FilesystemExecutor** — narrowly modify `worker_actions_executor_stubs.rs` to thread the policy through the executor; the stub remains "not implemented" but the policy module is fully testable in isolation. When the real impl lands, the policy is already there.
3. **Symlink escape defense** — `check_read` / `check_write` use `tokio::fs::canonicalize` (resolves symlinks) and verify the resolved path is still under `fs_root`. A symlink from inside `fs_root` pointing to `/etc/passwd` resolves out → denied.
4. **Worker config** — extend `manifests/workers/<persona>.toml` to declare `fs_root` (already a free TOML field today; document it). Real worker config types may be in `argo-vertical/src/manifest.rs` — **do NOT add a new schema field**; if `fs_root` is not yet recognized, keep it as a loose field read at executor-construction time and file an issue for proper typing.
5. **Audit log shape** — every check emits one `tracing::info!` event with target `argo::fs::audit`. A separate `claude-action-record-identity-plumbing` brief or future audit-store brief can ingest these into the audit store.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_fs_policy.rs` (NEW or extend; ≤ 380 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor_fs_policy_tests.rs` (NEW, ≤ 320 LOC sibling) — 10 unit tests:
  - read inside fs_root → allowed
  - read outside fs_root → denied with clear error
  - read of symlink escaping fs_root → denied
  - write inside fs_root → allowed
  - write outside fs_root → denied
  - read allowlist glob enforcement
  - write allowlist glob enforcement
  - file size cap enforcement
  - audit event emission shape (capture via `tracing-test`)
  - relative path resolution (relative to fs_root, not CWD)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line if NEW
- `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs` — narrowly: thread `FsSandboxPolicy` through the FilesystemExecutor stub constructor; ~20 LOC delta
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` — narrowly: in `resolve_executor` for `ExecutorKind::Filesystem`, build the policy from worker config and pass to the executor; ~15 LOC delta
- `crates/argo-cli/tests/fs_executor_sandbox_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration test asserting the full `resolve_executor → check_read` happy path AND a deny path
- `manifests/workers/<persona>.toml` (one example) — narrowly: document `fs_root` and allowlists; only if a real worker manifest already exists in the repo

## Files you must NOT touch

- `argo-vertical/src/manifest.rs` — DO NOT add a new schema field; rely on existing free-field TOML
- `argo-types::ActionOutcome`
- The `Executor` trait
- `argo-channels` / `argo-mcp` / `argo-store`
- Any other executor kind's dispatch file

## Acceptance criteria

- [ ] `FsSandboxPolicy::check_read` and `check_write` allow paths inside `fs_root`, deny outside
- [ ] Symlink escape (relative or absolute) is denied via canonicalization
- [ ] Read / write allowlist globs enforced
- [ ] File size cap enforced via `check_size`
- [ ] Audit event emitted for every check (target `argo::fs::audit`) with structured fields
- [ ] FilesystemExecutor stub now constructs with the policy threaded through (ready for real impl follow-up)
- [ ] Cross-tenant isolation: a worker's `fs_root` is the only path it can touch (assert via 2-worker fixture)
- [ ] 10 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-fs-executor-sandbox-policy`
- PR title: `actions(fs): sandbox policy with fs_root containment + audit (claude-fs-executor-sandbox-policy)`

## Notes for the implementer

- This brief does NOT make the FilesystemExecutor a real impl — that's a follow-up. It ships the policy + audit layer so the real impl plugs in with safety guarantees pre-baked.
- Per Tick 45 EXHAUSTIVE-MATCH: this brief does NOT add a new ExecutorKind variant, so workspace-wide match-arm changes are out of scope.
- Use `globset` (workspace dep candidate) for allowlist globs. If not yet a workspace dep, use simple `Path::starts_with` containment for v1 and file an issue for glob support.
- `tokio::fs::canonicalize` requires the path to exist for read checks. For write checks where the file may not exist yet, canonicalize the parent directory and join the file name — block any traversal in the canonicalized parent.
- The stub still returns `not implemented` from `dispatch`. The policy is threaded but doesn't gate anything until the real impl arrives. Tests focus on the policy in isolation.
- Audit events are `tracing::info!` only — no audit-store wiring. That's a separate brief (cf. `H-audit-query-extensions` already in streams/).
