# cu1-fence-scoped-fs-grant-for-file-tool-workers

**Owner**: claude — **SECURITY / capability-fence change → conductor adversarial-review before merge, do NOT self-merge.**
**Depends on**: none (smallest real-work unlock; foundation for the DEC-050 "real work" demos).
**Authored against**: origin/main `5f81ae776` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh cu1-fence-scoped-fs-grant-for-file-tool-workers

## Goal (DEC-050 G1)

Let a hired teammate do **real, fenced filesystem work in its own folder** —
read/write files under a worker-scoped `fs_root`, no shell, no host escape — so
"AI teammate does real work on the Mac" is demonstrable through the SPA within
the capability model.

## Why now (live e2e 2026-06-11, evidence in docs/skycot/demos/2026-06-11-computer-use-readiness-and-real-work-gap.md)

A Genesis teammate, asked to organize a folder, genuinely called the file
tools — and was correctly fenced: `rbac denied: genesis-worker lacks
FsRead { path_glob: … }` (and FsWrite, and Spawn for terminal). The fence
(`crates/argo-capability/src/worker_profile.rs::derive`) grants ONLY
`ToolDispatch` for the manifest's `declared_actions ∪ [tools].allowed`; it
emits **no FsRead/FsWrite** for any path. So a minted teammate cannot touch the
filesystem at all. Correct default — but there is no path to grant a teammate
its OWN sandboxed folder. This brief adds exactly that, minimally and safely.

## Fix (scoped, deny-by-default preserved)

1. **`worker_profile::derive`**: accept the worker's `fs_root: Option<&Path>`.
   When `fs_root` is `Some` AND `[tools].allowed` contains `file_read` →
   emit capability `FsRead{ path_glob: "<fs_root>/**" }`; for `file_write`/
   `file_patch` → `FsWrite{ path_glob: "<fs_root>/**" }`. NO fs_root, or tool
   not in allowed → emit nothing (unchanged default-deny). NEVER grant outside
   `fs_root`; NEVER grant `Spawn` (terminal stays owner-only — DEC-049).
2. **Thread `fs_root` → `ctx.workspace_root`** in the gateway run path so the
   file tools' jail (`file_write.rs` `refuses_absolute_path_outside_workspace`)
   coincides with the granted glob. Source: `worker.fs_root` (already a column
   on the `worker` table); thread `process_job` → `build_turn_dispatch_ctx`
   (`turn_helpers_run.rs`, the `cwd`/workspace param) → `ToolContext`. When
   `fs_root` is empty, behavior is unchanged (current cwd default).
3. **Hire/Genesis**: set a sensible per-worker `fs_root` at hire time (e.g.
   `~/SkycotWorkspaces/<persona>/`), created on first run. Keep it OFF the
   TCC-protected folders (not ~/Downloads, ~/Desktop, ~/Documents) so no OS
   permission is needed.

## May touch

- `crates/argo-capability/src/worker_profile.rs` (+ `worker_profile_tests.rs`)
- `crates/argo-agent-runner/src/turn_vertical_fence.rs` (derive call passes fs_root)
- `crates/argo-agent-runner/src/turn_helpers_run.rs` (workspace_root from fs_root)
- `crates/argo-cli/src/handlers/work_dispatch*.rs` (thread fs_root)
- hire path (genesis/template) to set `fs_root`

## Acceptance / verifier

1. Unit: `derive` with `fs_root=Some(/tmp/w)` + allowed `["file_write"]` →
   set covers `FsWrite{/tmp/w/**}` and does NOT cover `/tmp/other/**`, `Spawn`,
   or terminal `ToolDispatch`. With `fs_root=None` → no Fs caps (regression
   pin: default-deny preserved).
2. Live: a teammate with `fs_root` + file tools allowed reads real files in its
   fs_root and writes a real artifact there via Run-now — verifiable on disk +
   hash-chained audit row. A write OUTSIDE fs_root is still denied (negative
   pin).
3. `cargo test -p argo-capability -p argo-agent-runner -p argo-cli` green;
   clippy; no Spawn ever granted to a non-owner.

## Size / owner / priority

~120-200 LOC incl. tests. **🔴 P1 — DEC-050 keystone (smallest real-work
unlock).** Security-sensitive (loosens the fence, narrowly + scoped) →
adversarial review mandatory. Unblocks demo C (real fenced file work).
