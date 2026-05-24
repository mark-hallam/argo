# mission-delegate-task-fallback-chain-v2

**Owner**: codex.
**Depends on**: `mission-delegate-task-make-real` (MERGED).
**Authored against**: origin/main `9aded3f9a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-delegate-task-fallback-chain-v2`.

P1 claim-2 follow-up. The `delegate_task` tool now knows how to
inherit an active escalation chain when `ToolContext.current_handoff_id`
is set, but the agent-runner still leaves that field `None` on the live
dispatch path (`crates/argo-agent-runner/src/dispatch.rs`). The wire is
half-landed: tests inside `argo-tool` prove the inheritance helper, but
real turns dispatched from an inbound handoff still drop the caller's
chain.

## Scope

Plumb the current inbound handoff id through the runner's tool-dispatch
context so `delegate_task` can prepend the callee and carry forward the
active chain during normal worker turns. Keep the change narrow: no new
schema, no orchestration redesign, no policy fallback invention. This is
the missing runtime wire for the already-shipped inheritance helper.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: `crates/argo-agent-runner/src/dispatch.rs`

## Files you may touch

- `crates/argo-agent-runner/src/dispatch.rs`
- `crates/argo-agent-runner/src/turn_helpers_run.rs`
- `crates/argo-agent-runner/tests/**`

## Files you must NOT touch

- `crates/argo-tool/src/tools/delegate_task.rs` except for compile-fix-only signature adjustments required by the runner wire.
- `crates/argo-orchestrator/**`
- `crates/argo-identity/**`
- `docs/build-plan/BUILD_STATUS.md` except the standard PR-number row update.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] The live runner dispatch path sets `ToolContext.current_handoff_id` when servicing a turn that originated from an inbound handoff.
- [ ] `delegate_task` dispatched from that path emits a child handoff whose payload preserves the active escalation chain with the callee prepended.
- [ ] A no-handoff turn still leaves `current_handoff_id=None` and preserves current behavior.
- [ ] Coverage lands in `crates/argo-agent-runner/tests/**` rather than only relying on the existing `argo-tool` crate-local tests.
- [ ] `cargo test -p argo-agent-runner --tests` passes.
- [ ] `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` passes.
- [ ] Every modified file remains under 500 LOC.

## Branch + PR

- Branch: `codex-mission-delegate-task-fallback-chain-v2`
- PR title: `runner: wire delegate_task fallback-chain inheritance (mission-delegate-task-fallback-chain-v2)`

## Notes for the implementer

- Start by finding the runner path that already knows which inbound handoff is being serviced; thread only that id.
- Do not expand scope to policy-default fallback logic in this brief. The goal is to make the already-landed active-handoff inheritance reachable.
