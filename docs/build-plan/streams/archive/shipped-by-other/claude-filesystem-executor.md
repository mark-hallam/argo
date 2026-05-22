# V1.2 — Filesystem ExecutorKind

**Owner**: claude. **Depends on**: A-executor-trait (Stream A
on main).

## Scope

New `ExecutorKind::Filesystem` for sandboxed file operations.
The mission says "any task a human can do on a PC/Mac";
filesystem ops are a foundational PC/Mac capability. v1
ships ComputerUse (browser) only.

Sandboxing rules:
- Per-worker root prefix configured at spawn:
  `worker.fs_root = /tmp/argo-fs/<worker_id>/`
- Path traversal (`..`, absolute paths outside root)
  rejected hard
- Per-op size cap (default 16 MiB; env-overridable
  `ARGO_FS_OP_SIZE_CAP`)
- Per-worker total-size cap (default 1 GiB)
- Symlinks not followed past the root

Operations exposed:
- `read_file(path) -> bytes`
- `write_file(path, bytes)`
- `list_dir(path) -> Vec<DirEntry>`
- `mkdir(path)`
- `remove(path)` — single file/dir, never recursive (high-
  risk would need recursive flag + approval)
- `stat(path) -> FileMeta`

All approval-gated by default; read-only ops auto-approved
under existing AutoApprovalPolicy.

## Files you may touch
- EXHAUSTIVE-MATCH amendment applies (adding `ExecutorKind`
  variant transitively allows touching matches)
- `crates/argo-runtime/src/executors/filesystem.rs` (NEW,
  ~500 LOC cap)
- `crates/argo-runtime/src/executors/mod.rs` — narrowly:
  register `Filesystem` variant
- `crates/argo-identity/src/executor_kind.rs` — narrowly:
  add `Filesystem` arm
- `crates/argo-store/migrations/<next>__worker_fs_root.sql`
  (NEW) — adds `fs_root` column to workers
- `crates/argo-cli/src/handlers/worker_spawn.rs` — narrowly:
  optional `--fs-root` flag

## Acceptance criteria
- [ ] All 6 ops execute in sandbox
- [ ] Path traversal rejected
- [ ] Size caps honored (per-op + per-worker total)
- [ ] Symlinks not followed past root
- [ ] Read ops auto-approved; write/delete approval-gated
- [ ] `worker.fs_root` persisted + reloaded
- [ ] 14 unit tests (each op + sandbox-violation) + 2
      integration tests
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-filesystem-executor`
- PR title: `runtime: sandboxed filesystem executor (claude-filesystem-executor)`
