# fix-fs-executor-reject-symlink-dir-components

**Authored against:** `origin/main` 6d148f0e1585b73b9527ef3ed8a0201d27eb4aca on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-fs-executor-reject-symlink-dir-components`.

## Why this brief exists

Surfaced as the FINDINGS pin from PR #2124 (`eval-filesystem-executor-fs-root-enforced`, MERGED 2026-05-23T10:50Z). The PR pins 13 cross-tenant contracts and adds 1 `#[ignore]`'d FINDINGS test at `crates/argo-executor-filesystem/tests/fs_root_cross_tenant.rs`:

> Writing through a path where a **directory component** is a symlink pointing outside `worker_fs_root` succeeds. The Sandbox/ops layer canonicalises only the final filename — a `dir-symlink/escapeme` write reaches outside the worker's fs root.

This is a real per-worker isolation hole: a malicious or compromised vertical pack could plant a dir-symlink during a first turn, then exfil through it on a second turn.

## Acceptance criteria

1. Identify the canonicalisation site in `crates/argo-executor-filesystem/src/` (Sandbox::open / write helpers) and harden so every **directory component** in the resolved path lives within `worker_fs_root`, not just the final basename. Use `Path::canonicalize` on the full chain OR walk components rejecting any `is_symlink()` that resolves outside root.
2. Un-`#[ignore]` the FINDINGS test in `crates/argo-executor-filesystem/tests/fs_root_cross_tenant.rs` and confirm it passes (write-through-dir-symlink is REJECTED).
3. Add 1 read-side companion test: `read_through_dir_symlink_escape_rejected` (same dir-symlink shape, read path).
4. Per-crate `cargo clippy -p argo-executor-filesystem -- -D warnings` clean.

## Parallel-surface declarations

- Extends existing Sandbox/ops layer in `crates/argo-executor-filesystem/src/`. No new pub surface.
- Wire-up consumer: already wired via `crates/argo-cli/src/handlers/worker_actions_executor_stubs_filesystem.rs` (per the PR #2124 contract pin).

## May-touch

- `crates/argo-executor-filesystem/src/*` (sandbox / open / write helpers)
- `crates/argo-executor-filesystem/tests/fs_root_cross_tenant.rs` (un-`#[ignore]` + 1 new test)

## Boundary

- Do NOT change the `worker_fs_root` config shape or the per-worker isolation primitive contract — only the canonicalisation step.
- TOCTOU (time-of-check-to-time-of-use) full hardening is out of scope; component-level symlink rejection at open time is the bar.

## Sizing

~80-180 LOC. Not fast-mode (needs careful path-component logic).

**🟡 P2 CLAIM-5 (per-worker isolation hygiene)** — claim-5 already LIVE on main via #2112 happy-path; this closes a real audited escape vector and turns the FINDINGS pin green.
