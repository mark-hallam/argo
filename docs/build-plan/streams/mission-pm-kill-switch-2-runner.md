# Stream P2-12-2 — kill switch: agent-runner pre-turn + pre-dispatch checks

**Owner**: codex.
**Depends on**: mission-pm-kill-switch-1-store-audit MERGED.
**Authored against**: origin/main `6bcb9aeb` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-kill-switch-2-runner`.
**Source**: skycot-pm-worker REPORT.md §6.2 + §11 P2 #12. Split slice 2/5 of parent `mission-pm-kill-switch` (BLOCKED 2026-05-19 — incomplete may-touch).

## Scope

The runner half of the worker-freeze primitive. Calls `worker_freeze::is_frozen(worker_id)` from two places:

1. **Pre-turn**: before the turn loop starts in `turn.rs`. If frozen, abort with a clear error message and emit no further work.
2. **Pre-dispatch**: before each tool dispatch in `turn_helpers.rs` (the actual dispatch loop). If frozen, refuse the dispatch and emit a `WorkerFreezeBlocked` audit row. In-flight tool calls already in progress are NOT cancelled — this is cooperative.

Cache the result of `is_frozen` once per turn — do not re-query the DB on every tool call within a single turn. A freeze applied mid-turn lands at the next turn boundary (acceptable v1 behaviour).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-agent-runner/src/turn.rs` — narrowly scoped: before the turn loop starts, call `is_frozen(worker_id)`. If `Some(rec)`, return `ArgoError::Invariant("worker frozen: ...")` with the message format documented in Notes. ≤ 20 LOC of additions.
- `crates/argo-agent-runner/src/turn_helpers.rs` — narrowly scoped: in the dispatch loop, before invoking the next tool call, check the cached `is_frozen` result. If frozen, write a `WorkerFreezeBlocked` audit row and short-circuit the loop with the same error type. ≤ 30 LOC of additions including the per-turn cache.
- `crates/argo-agent-runner/src/dispatch.rs` — if there is a sibling dispatch entry point, mirror the pre-dispatch check there. ≤ 15 LOC if needed; may be unnecessary if all dispatches funnel through `turn_helpers.rs`.
- `crates/argo-agent-runner/src/turn_tests.rs` — add 2 tests: (a) frozen worker cannot start a turn (error message matches the documented format); (b) freeze applied mid-turn-loop blocks the next dispatch and emits the audit row.
- `crates/argo-agent-runner/src/dispatch_tests.rs` — add 1 test: pre-dispatch check refuses tool calls for a frozen worker.

## Files you must NOT touch

- `crates/argo-store/**` — the store primitive is owned by sibling 1.
- `crates/argo-audit/**` — `WorkerFreezeBlocked` variant is owned by sibling 1.
- `crates/argo-cli/**` — CLI lands in sibling 3.
- MCP / tool subprocess code — this is cooperative, not preemptive.
- `crates/argo-tool/src/registry.rs` dispatch — check lives in the agent-runner pre-dispatch hook only.

## Acceptance criteria

- [ ] Pre-turn check refuses to start a turn for a frozen worker; error message includes the freeze reason.
- [ ] Pre-dispatch check refuses tool calls and emits a `WorkerFreezeBlocked` audit row.
- [ ] The `is_frozen` result is cached once per turn — verified by a test that asserts only one `worker_freeze` DB query per turn even with multiple tool calls.
- [ ] In-flight tool calls already in progress are NOT cancelled (verify by a test where dispatch returns successfully if the freeze landed AFTER dispatch started).
- [ ] `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-agent-runner turn::tests dispatch::tests` passes.
- [ ] `cargo test -p argo-agent-runner` overall passes (or only pre-existing flakes from KNOWN_FLAKES.md fail).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-kill-switch-2-runner`
- PR title: `feat(agent-runner): pre-turn + pre-dispatch freeze checks (mission-pm-kill-switch-2-runner)`

## Notes for the implementer

- Error message format (user-facing): `"Worker {worker_id} is frozen by {frozen_by}: {reason}. New turns and tool calls are blocked. In-flight tool calls continue to completion."` — match this exactly so the CLI / UI siblings can render it without translation.
- Cache the `is_frozen` result on the per-turn context object. If your `turn_helpers.rs` doesn't have a per-turn context yet, the smallest expedient is a local `Option<Option<FreezeRecord>>` at the top of the dispatch loop initialised on first check — do NOT introduce a new per-turn struct.
- A freeze applied mid-turn is observed at the next dispatch boundary. This is intentional — preemptive cancellation is a separate brief.
- The dispatch-loop tests should drive the freeze via the existing `dispatch_one_with_tenant` path so the test exercises real concurrency, not a mocked check.

## Out of scope

- Store primitive (sibling 1 — must be MERGED first).
- CLI commands (sibling 3).
- Gateway routes (sibling 4).
- UI button (sibling 5).
- Preemptive SIGKILL — separate brief.
