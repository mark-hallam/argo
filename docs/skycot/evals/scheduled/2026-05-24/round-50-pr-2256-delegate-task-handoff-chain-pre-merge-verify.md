# Round 50 — PR #2256 `mission-delegate-task-fallback-chain-v2` PRE-MERGE anti-theater verify

- **Date**: 2026-05-24 (Melbourne 2026-05-25 06:1x AEST)
- **Round**: 50
- **Mission claim covered**: Claim 2 (Delegation-as-unit) — first claim-2 coverage in 6 rounds
- **Verdict**: 🟢 PASS · severity tier P0 (load-bearing claim-2 runtime contract)

## Setup

PR head: `a76afadc9ad51a4d5b6c57273bf346bc88d6ae98`, branch `codex-mission-delegate-task-fallback-chain-v2`.

```
git worktree add /tmp/pr-2256-verify -b verify-2256-r50 origin/codex-mission-delegate-task-fallback-chain-v2
cd /tmp/pr-2256-verify
CARGO_TARGET_DIR=/tmp/pr-2256-verify-target cargo test -p argo-agent-runner --test delegate_task_handoff_chain_runtime
```

Cold build: 11m 44s · test run: 0.29s.

## Observed

```
running 2 tests
test delegate_task_without_inbound_handoff_leaves_chain_unset ... ok
test delegate_task_inherits_active_inbound_handoff_chain ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
```

Production diff (3 files, +336 / -7 LOC):

- **`crates/argo-agent-runner/src/dispatch.rs:194-209`** — `dispatch_one_with_runtime` gains a new `current_handoff_id: Option<HandoffId>` parameter; populates `ctx.current_handoff_id = current_handoff_id;` on the freshly constructed `ToolContext`. The previous hardcoded comment "`current_handoff_id` stays `None` here" is deleted.
- **`crates/argo-agent-runner/src/turn_helpers_run.rs:45,118-161,366`** — `TurnDispatchCtx` gains `current_handoff_id: Option<HandoffId>` field; `build_turn_dispatch_ctx` calls new helper `current_inbound_handoff_id(orchestrator, tenant_id, session_id, worker_id)`. Helper queries `orchestrator.list_handoffs_for_session(tenant, session)`, reverses iteration, finds the first whose status is `Pending|Acknowledged` AND `to == EscalationStep::Worker(this_worker)`. Threaded to `dispatch_one_with_runtime` at line 366.
- **`crates/argo-tool/src/tools/delegate_task.rs:163,204-219`** (read on the branch) — `inherit_active_chain` reads `ctx.current_handoff_id.as_ref()`, fetches the active handoff, copies its active escalation chain, and prepends the callee worker.

## Anti-theater check

Both tests are end-to-end runtime tests (use `run_turn_with_events` with a `wiremock` Anthropic mock), not unit tests on the helper alone. They exercise the full plumbing: helper → `TurnDispatchCtx` → `dispatch_one_with_runtime` → `ToolContext.current_handoff_id` → `delegate_task` tool → `inherit_active_chain`.

**Load-bearing assertion #1** — `delegate_task_inherits_active_inbound_handoff_chain`:

```rust
assert_eq!(
    emitted.payload.get("escalation_chain"),
    Some(&serde_json::json!([
        format!("worker:{}", f.callee),
        format!("worker:{}", f.receiver),
        format!("human:{HUMAN_FALLBACK}"),
    ]))
);
assert_eq!(emitted.payload.get("escalation_index"), Some(&serde_json::json!(0)));
```

Would FAIL on `origin/main`: the old `dispatch.rs:209-213` hardcoded `current_handoff_id: None`, so `inherit_active_chain` would short-circuit at `let Some(active_id) = ctx.current_handoff_id.as_ref() else { return Ok(()); }` and never write `escalation_chain` / `escalation_index` to the payload. The test's assertion `Some(json!([...]))` would not match `None`.

**Load-bearing assertion #2** — `delegate_task_without_inbound_handoff_leaves_chain_unset`:

```rust
assert!(emitted.payload.get("escalation_chain").is_none());
assert!(emitted.payload.get("escalation_index").is_none());
```

Negative control. Pins the no-handoff baseline so a future regression that accidentally synthesizes a chain when there's no inbound parent would FAIL.

The wiremock provider script asserts `delegate_task` is called as the first turn (call #0 returns `tool_use`); call #1 receives the tool_result back and the test asserts the result is non-empty (the emitted handoff id flows through). The 2-turn budget pins that this is exactly the happy path — no extra spurious turns.

## Honesty signal

PR body discloses a pre-existing test failure outside the brief's may-touch list:

> `cargo test -p argo-agent-runner --tests` — failed on existing `turn::turn_skills_tests::legacy_worker_treated_as_lowest_grade`; 78 passed, 1 failed. The same test fails standalone with no integration tests running.

Confirmed: `crates/argo-agent-runner/src/turn_skills_tests.rs:98` defines this test; it is NOT in `docs/build-plan/KNOWN_FLAKES.md`. Pre-existing main-branch drift, not introduced by #2256. Anti-theater-honest disclosure (matches r47/r48/r49 codex hygiene baseline).

## Gap / follow-up brief candidates

- **`fix-agent-runner-legacy-worker-treated-as-lowest-grade-test` (P3, ~30-80 LOC)** — pre-existing test failure pre-dating #2256 not tracked in `KNOWN_FLAKES.md`. Brief should first verify on `origin/main` HEAD (`cargo test -p argo-agent-runner --lib turn_skills_tests::legacy_worker_treated_as_lowest_grade --no-run` then run), then either fix the test or the production code OR register it in `KNOWN_FLAKES.md`. Not demo-blocking.

## Verdict signal for next conductor tick

**Recommend admin-merge next claude-conductor tick.** #2256 closes the claim-2 delegation-chain runtime plumbing gap that has been TODO since the original brief (`mission-delegate-task-fallback-chain` — first version). 29th anti-theater-honest substrate contract verification this week. Tests are load-bearing on real production logic; would fail on `origin/main`.
