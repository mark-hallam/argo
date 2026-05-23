# mission-path-guard-canonicalize-symlinks

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `a3a881fa3` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-path-guard-canonicalize-symlinks`.

🟡 **P2 CLAIM-5 KNOWN-GAP.** `argo-tool::path_guard::confine_path` is lexical-only — in-workspace symlinks escape. Documented at `crates/argo-tool/src/path_guard.rs:30-32` (#677 deferred).

## Why

PR #2117 (eval-argo-tool-surface-beyond-15-in-core, MERGED) finding F3 — pinned by `path_guard_rejects_symlink_inside_workspace_today` (currently `#[ignore]`'d) which demonstrates the escape under `--ignored`. With ai-receptionist scenarios increasingly running on workspaces with symlinks (npm/pnpm/cargo target dirs all symlink), the lexical-only guard is one rogue agent away from a containment break.

## Scope

- Replace lexical `confine_path` with canonicalize-then-prefix-check (`Path::canonicalize()` + `starts_with`).
- Handle symlink chains and non-existent-tail components (canonicalize the deepest existing ancestor + lexical-join the tail).
- Decide policy on missing-target symlinks (recommend reject + cite test).
- Un-ignore the FINDINGS-ONLY pin test from #2117.
- New tests: in-workspace symlink to outside-workspace target → reject; in-workspace symlink to in-workspace target → allow; symlink-of-symlink chain → follow + check.
- ~150-250 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-tool/src/path_guard.rs`
- `crates/argo-tool/tests/argo_tool_substrate_pin_test.rs` (un-ignore the F3 pin)
- New tests under `crates/argo-tool/tests/`
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Callers of `confine_path` (likely behaviour-compat, but if a caller's invariants change, name a separate brief).
- Other crates.

## Acceptance criteria

- [ ] Symlinks escaping the workspace are rejected.
- [ ] Existing safe-path behaviour preserved (no false positives in normal workspace operation).
- [ ] FINDINGS-ONLY pin from #2117 un-ignored + passing.
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
