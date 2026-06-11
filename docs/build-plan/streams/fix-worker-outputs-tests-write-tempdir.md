# fix-worker-outputs-tests-write-tempdir

**Owner**: claude or codex — test hygiene; conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `7951a48f1` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-worker-outputs-tests-write-tempdir

## Problem (root cause of the 2026-06-12 00:10-04:58 worker stall)

The worker-outputs tests (`routes_worker_outputs_tests.rs` + the run-exec
bundle writers they exercise, landed via #2830) write artifact bundles to
`.skycot/outputs/<job>/` **relative to the test cwd**, dropping dirs into
`crates/argo-cli/`. The strays tripped the worker-loop dirty-tree claim guard
(5x fast-fail → cooldown → claim deadlock; 4 briefs stalled ~5h). Mitigated by
gitignoring `.skycot/` — but tests writing into the source tree is the bug.

## Scope

In the tests (NOT production code): point the bundle root at a `tempfile::tempdir()`
via whatever seam the bundle writer already exposes (env var / config field —
grep `\.skycot/outputs` in `crates/argo-cli/src/handlers/gateway/` for the
root resolution; add a test-only override seam ONLY if none exists, mirroring
how `ARGO_HOME` is injected in gateway spawn tests). Assert no `.skycot/`
appears under the crate dir after the suite (one canary test).

## Acceptance

- `cargo test -p argo-cli --bin argo is_404` (and the outputs suite) leaves
  `git status --porcelain crates/argo-cli/` EMPTY.
- Production bundle path behavior unchanged (the gitignore stays regardless).
