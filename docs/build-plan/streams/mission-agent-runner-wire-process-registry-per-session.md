# mission-agent-runner-wire-process-registry-per-session

**Owner**: claude.
**Depends on**: `mission-process-registry-tenant-keyed` MUST land first (order-dependency).
**Authored against**: origin/main `a3a881fa3` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-agent-runner-wire-process-registry-per-session`.

🟡 **P1 CLAIM-5 BROKEN-WIRE.** `terminal { background: true }` + the `process` tool are dead in production because `ToolContext.background_processes` is never set by `argo_agent_runner::build_dispatch_ctx`. The tools dispatch, but every spawn returns a registry that is dropped at end-of-call.

## Why

PR #2117 (eval-argo-tool-surface-beyond-15-in-core, MERGED) finding F1 — pinned by `production_tool_context_lacks_process_registry_today` (currently `#[ignore]`'d). Worker turns silently lose background-process tracking.

## Scope

- In `argo_agent_runner::build_dispatch_ctx` (or equivalent constructor), instantiate / fetch a per-session `ProcessRegistry` keyed by `(tenant_id, session_id)`.
- Wire it into `ToolContext.background_processes` before dispatch.
- Lifecycle: clean up registry entries on session end (or rely on tenant-keyed shutdown).
- Un-ignore the `production_tool_context_lacks_process_registry_today` pin test from #2117.
- New positive integration test: agent turn spawns a `terminal { background: true }` → subsequent `process` tool call sees the spawned PID → kill via `process` tool succeeds.
- ~250-400 LOC across `argo-agent-runner` + tests.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-agent-runner/src/turn_helpers.rs` (or wherever `build_dispatch_ctx` lives — grep at claim-time)
- `crates/argo-tool/tests/argo_tool_substrate_pin_test.rs` (un-ignore the F1 pin)
- New integration test file under `crates/argo-agent-runner/tests/`
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `ProcessRegistry` internals — owned by sibling brief.
- Other crates outside may-touch.

## Acceptance criteria

- [ ] Background processes survive across turns within a session.
- [ ] Registry is tenant-scoped (verified by negative test: tenant_b cannot see tenant_a's PIDs).
- [ ] FINDINGS-ONLY pin from #2117 un-ignored + passing.
- [ ] `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
