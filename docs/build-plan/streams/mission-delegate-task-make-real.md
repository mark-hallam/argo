# mission-delegate-task-make-real — precursor: wire delegate_task stub into a real handoff-emitting tool

**Owner**: claude. **Depends on**: existing `OrchestratorStore::insert_handoff` API; existing `Handoff` struct.
**Authored against**: origin/main `6676b391` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-delegate-task-make-real`.

> **PRECURSOR 2026-05-13 by conductor (CEO-authorized Cluster D #3).** Unblocks `mission-delegate-task-fallback-chain` v2. The `delegate_task` tool today is a stub returning "subagent delegation not wired" — nothing's calling it because it doesn't work. Make it real: emit a Pending handoff to the target worker + plumb session/handoff context into `ToolContext` so the downstream fallback-chain brief can inherit chain semantics.

## Scope

1. **Replace the stub** in `crates/argo-tool/src/tools/delegate_task.rs::dispatch` (currently line 54) with a real implementation that:
   - Reads target `worker_id` from the tool args.
   - Reads caller context from `ToolContext` (extended below).
   - Calls `OrchestratorStore::insert_handoff` to create a Pending handoff.
   - Returns the new handoff id to the agent loop.

2. **Extend `ToolContext`** (currently `crates/argo-tool/src/tool.rs:51-87`) with the fields downstream briefs need:
   - `pub session_id: SessionId`
   - `pub current_handoff_id: Option<HandoffId>` (the inbound handoff the caller is servicing, if any)
   - These are additive fields; existing callers compile with `..Default::default()` or struct-update syntax.

3. **Plumb the new fields** at every `ToolContext::new` call site (typically `agent-runner::dispatch` and any test helpers). Most call sites already have `session_id` in scope; pulling `current_handoff_id` from the orchestrator state is straightforward.

4. **The actual escalation-chain inheritance is NOT in this brief.** That's `mission-delegate-task-fallback-chain-v2` (the rewritten downstream brief). This brief just makes the wires exist.

## Files you may touch

- `crates/argo-tool/src/tools/delegate_task.rs` — replace stub with real handoff-emitting impl. ≤200 LOC.
- `crates/argo-tool/src/tools/delegate_task_tests.rs` — narrowly: 5 unit tests (success, missing-target, invalid-worker, no-session-context, deadline-set).
- `crates/argo-tool/src/tool.rs` — narrowly: add 2 additive fields on `ToolContext`. ≤15 modified lines.
- `crates/argo-tool/Cargo.toml` — narrowly: add `argo-orchestrator` dep if not already present (likely a workspace dep).
- `crates/argo-agent-runner/src/dispatch.rs` (or wherever `ToolContext::new` is called) — narrowly: populate the 2 new fields. ≤20 modified lines per call site.
- `<NEW: crates/argo-tool/tests/delegate_task_real_smoke.rs>` — integration smoke: agent calls delegate_task → orchestrator-store contains the new Pending handoff.

## Files you must NOT touch

- `crates/argo-orchestrator/src/handoff.rs` — `Handoff` struct + `insert_handoff` are locked.
- `crates/argo-identity/src/policy.rs` — `EffectivePolicy` is downstream-brief territory.
- The escalation-chain logic — downstream brief.

## Acceptance criteria

- [ ] `delegate_task::dispatch` returns `Ok(handoff_id)` on the happy path (was: `Err("not wired")`).
- [ ] `ToolContext` carries `session_id` + `current_handoff_id` additively.
- [ ] Every `ToolContext::new` call site compiles with the new fields populated.
- [ ] 5 unit tests for the tool + 1 integration smoke.
- [ ] Reusing an existing `OrchestratorStore::insert_handoff` — NO new orchestrator API.
- [ ] `cargo clippy -p argo-tool -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-delegate-task-make-real`
- PR title: `tool: real delegate_task with handoff emission + ToolContext plumbing (mission-delegate-task-make-real)`

## Notes for the implementer

- This brief is the **prerequisite** for fallback-chain inheritance. Don't try to add chain logic here — that brief lands separately.
- For `current_handoff_id`: agent-runner already knows what handoff (if any) the current turn is servicing. Plumb from there. If no inbound handoff, pass `None`.
- Once this lands, Mission Claim 2 (delegation-as-a-unit) becomes implementable instead of decorative.
