# Round 54 — PR #2256 `mission-delegate-task-fallback-chain-v2` PRE-MERGE anti-theater verify

- **Date**: 2026-05-25 (UTC) · Melbourne 2026-05-25 ~10:30 AEST
- **Round**: 54 (scheduled-task hourly fire ~1h20min after r53 ship)
- **Pillar / Claim**: Claim 2 (Delegation-as-unit) + cross-pillar Claim 3 (Platform builds workers) — the runtime wire that lets a chained `delegate_task` inherit the in-flight inbound handoff's escalation chain
- **Verdict**: 🟢 **PASS — PRE-MERGE CLEAN, MERGEABLE** (P0 — load-bearing runtime gap; CLAUDE.md status comment in old `dispatch.rs:210` explicitly named this brief as the missing wire)

## Setup

- `gh pr view 2256 --json title,additions,deletions,changedFiles,body` — title `runner: wire delegate_task fallback-chain inheritance (mission-delegate-task-fallback-chain-v2)`, 3 files +336/-7, brief id `mission-delegate-task-fallback-chain-v2`
- `gh pr diff 2256` — full diff read (3 files)
- `rg -nF "current_handoff_id" crates/argo-tool/src/` — verify consumer side
- `rg -nF "current_handoff_id" crates/argo-agent-runner/src/` — verify producer side / pre-fix-comment line

## Observed

### Production wire (the seam that didn't exist before)

`crates/argo-agent-runner/src/turn_helpers_run.rs` grows a new helper:

```rust
async fn current_inbound_handoff_id(
    orchestrator: &argo_orchestrator::OrchestratorStore,
    tenant_id: TenantId,
    session_id: &argo_orchestrator::SessionId,
    worker_id: Option<argo_store::WorkerId>,
) -> ArgoResult<Option<HandoffId>> {
    let Some(worker_id) = worker_id else { return Ok(None); };
    let handoffs = orchestrator.list_handoffs_for_session(tenant_id, session_id).await?;
    Ok(handoffs.into_iter().rev().find_map(|handoff| {
        let active = matches!(handoff.status, HandoffStatus::Pending | HandoffStatus::Acknowledged);
        let assigned_to_worker =
            matches!(&handoff.to, EscalationStep::Worker(id) if id == &worker_id);
        (active && assigned_to_worker).then_some(handoff.id)
    }))
}
```

This is called at `build_turn_dispatch_ctx` and the result threaded through `TurnDispatchCtx::current_handoff_id` → `resolve_tool_call` → `dispatch_one_with_runtime(...)` (which grows a new param) → `ToolContext::current_handoff_id`.

`crates/argo-agent-runner/src/dispatch.rs:207-213` (old) — the TODO-comment-as-stub:

> `// `current_handoff_id` stays `None` here; the upstream agent loop does not yet track which inbound handoff (if any) is being serviced this turn. `mission-delegate-task-fallback-chain-v2` is the brief that plumbs it.`

…is **deleted** in this PR, and replaced with a real assignment from the new `current_handoff_id: Option<HandoffId>` parameter.

The consumer at `crates/argo-tool/src/tools/delegate_task.rs:204-224` reads `ctx.current_handoff_id`, walks `OrchestratorStore::find_handoff` for the active inbound, and prepends the callee to the active escalation chain — the production behavior the test pins.

### Test pin (`crates/argo-agent-runner/tests/delegate_task_handoff_chain_runtime.rs` — 297 LOC new)

**Two `#[tokio::test]` cases**:

1. **`delegate_task_inherits_active_inbound_handoff_chain`** — inserts an active handoff with `EscalationStep::Worker(f.receiver)` and chain `[Worker(receiver), Human(ops@example.test)]` at index 0, drives a real `run_turn_with_events` call against a `wiremock` provider that returns a `delegate_task` tool_use, then asserts:

   ```rust
   assert_eq!(
       emitted.payload.get("escalation_chain"),
       Some(&serde_json::json!([
           format!("worker:{}", f.callee),    // callee PREPENDED
           format!("worker:{}", f.receiver),  // active inbound's chain preserved
           format!("human:{HUMAN_FALLBACK}"), // fallback preserved
       ]))
   );
   assert_eq!(emitted.payload.get("escalation_index"), Some(&serde_json::json!(0)));
   ```

2. **`delegate_task_without_inbound_handoff_leaves_chain_unset`** — same harness, NO active handoff inserted; asserts the emitted handoff has **no** `escalation_chain` key (per the early-return at `inherit_active_chain` when `ctx.current_handoff_id.is_none()`).

## Anti-theater check

Five independent levers prove the test would FAIL loudly if the wire regressed:

1. **The exact 3-element chain assertion** in the positive test. If `current_inbound_handoff_id` returned `None` despite the active handoff existing, `inherit_active_chain` would early-return at line 210 (`let Some(active_id) = ctx.current_handoff_id.as_ref() else { return Ok(()); };`), and `emitted.payload.get("escalation_chain")` would be `None` — assertion fails on `Some(...)` vs `None`.
2. **The negative test** would pass under a broken wire too (vacuous), so the **positive test is the real defence**.
3. The helper's filter is **doubly-conditional** (`active && assigned_to_worker`) — a vacuous return of any handoff would NOT pass the positive test, because the production code uses the returned id to call `store.find_handoff(...)` and reads its `active_escalation_chain()` — the test inserted exactly one such handoff with a non-empty chain, so finding the wrong one would yield a different chain layout.
4. **`run_turn_with_events`** is the real production entry point used by the live runner — not a unit-test scaffold. The `wiremock` provider returns canned Anthropic-format tool_use responses, exercising the full turn loop including capability checks + audit emission paths.
5. **The brief id `mission-delegate-task-fallback-chain-v2`** is the literal id referenced in the deleted dispatch.rs comment — semantic continuity proves this is the intended wire-up, not a parallel-surface drift.

## PR shape clean

- 3 files: `dispatch.rs` (+2/-5), `turn_helpers_run.rs` (+34/-1), `tests/delegate_task_handoff_chain_runtime.rs` (+297/-0 new)
- No new pub types (extends existing `ToolContext` field that already had type `Option<HandoffId>` at `argo-tool/src/tool.rs:101`)
- No parallel surface (this IS the canonical wire — TODO comment named this brief)
- 4 prior unit-test callsites at `argo-tool/src/tools/delegate_task_tests.rs:{239,273,293,309}` pre-existed (in-crate tests poked `ctx.current_handoff_id = Some(...)` manually) — this PR adds the first **integration**-level pin where the wire is exercised through `run_turn_with_events`

## Gap / follow-up brief candidates

**None this scenario.** PR is clean MERGEABLE. Identical structural shape to r52 #2260 (skill query tenant-bind) and r53 #2261 (team-headline tenant-bind) — small surface, clear consumer, real test pin with explicit failure-mode assertion.

**Recommendation**: Add #2256 to the next merge wave alongside #2244 / #2248 / #2257 / #2259 / #2260 / #2261 / #2262 / #2263 once the LOC-budget gate is unblocked. **This is the 37th anti-theater-honest verification this week** — pattern stays steady; the only failure mode is the LOC-budget gate blocking the entire queue.
