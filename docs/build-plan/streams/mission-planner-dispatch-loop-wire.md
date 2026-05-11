# Mission load-bearing wire — wire `Planner::decompose` into agent dispatch loop

**Owner**: claude. **Depends on**: `argo-runtime::planner::Planner` trait SHIPPED (verified at `crates/argo-runtime/src/planner.rs:184`); `LlmPlanner` impl SHIPPED (`planner.rs:198`); `record_decomposition` helper SHIPPED (`planner.rs:48`); `SubtaskSink` impl exists in argo-store (per rustdoc note at planner.rs:30-33).
**Authored against**: origin/main `e44f6e8a` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-planner-dispatch-loop-wire`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 2 #50 — "Wire planner into agent dispatch loop." Today the `Planner::decompose` API exists end-to-end but is ONLY invoked from a dedicated CLI handler (`argo plan decompose` style). When a worker turn produces a top-level goal that is decomposable, the agent dispatch loop does NOT consult the planner — it just executes the goal as a single subtask. Mission promise: dispatch detects "decomposable goal" intent + auto-invokes the planner + writes children via `record_decomposition` + the existing handoff/walker machinery resumes the children.

## Scope

1. **First 5min investigation**: trace `argo-agent-runner::turn` to find where a goal is dispatched as a tool call OR resolved to a single executor step. Identify the "should this be decomposed?" decision point. Cite file + line in PR body.

2. **Add a NEW trait**: `argo-runtime::dispatch::DecomposeGate`:
   ```rust
   #[async_trait::async_trait]
   pub trait DecomposeGate: Send + Sync {
       /// Decide whether `goal` warrants a planner.decompose pass.
       /// Returns Ok(true) → call planner; Ok(false) → skip + dispatch as single task.
       async fn should_decompose(&self, goal: &str, ctx: &PlannerContext) -> Result<bool, GateError>;
   }
   ```
   First v1 impl: `HeuristicGate` — returns `true` when the goal contains explicit list markers ("and then", `\n- `, `;`, " then ") OR length > 200 chars. Keep heuristic simple + deterministic + unit-testable.

3. **At dispatch time** (the call site identified in step 1):
   - Call `gate.should_decompose(goal, &ctx)`.
   - If `false` → existing behavior (single task).
   - If `true` → call `Planner::decompose(goal, &ctx)` → call `record_decomposition(sink, ...)` → emit a `tracing::info!` event `"goal_decomposed parent=... children=N"`.
   - Existing walker/handoff loop picks up the child subtasks via the SubtaskSink rows.

4. **Failure modes**:
   - planner.decompose errors → log `tracing::warn!`, fall back to single-task dispatch (don't crash).
   - record_decomposition errors → log only.
   - gate.should_decompose errors → assume false (don't decompose, don't crash).

5. **Wiring**: agent-runner constructs `Box<dyn DecomposeGate>` + `Arc<dyn Planner>` + `&dyn SubtaskSink` through `AgentDeps`. Tests inject a `MockPlanner` that returns a known `DecomposeResult`.

This is NOT in scope: LLM-backed gate (HeuristicGate only for v1; `mission-planner-llm-gate` follow-up); re-decompose on partial failure; multi-level recursion limits (use existing recursion guard if present, else flag + TODO); changing `Planner::decompose` signature.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-runtime/src/dispatch.rs` OR `dispatch_gate.rs` (NEW, ≤ 200 LOC) — `DecomposeGate` trait + `HeuristicGate` impl + `GateError`. Sibling module keeps `lib.rs` clean.
- `crates/argo-runtime/src/dispatch_gate_tests.rs` (NEW, ≤ 150 LOC) — 6 unit tests: short-simple-goal-no-decompose; goal-with-and-then-decomposes; goal-with-bullets-decomposes; long-prose-decomposes; empty-goal-no-decompose; semicolon-list-decomposes.
- `crates/argo-runtime/src/lib.rs` — narrowly scoped: ADD module declaration + re-export.
- `crates/argo-agent-runner/src/turn.rs` OR `turn_helpers.rs` (verify in first 5min) — narrowly scoped: ONE new call site at the dispatch-decision point. ≤ 40 LOC delta. If it pushes file over 500 LOC, spill to sibling `turn_planner.rs` (NEW, ≤ 150 LOC).
- `crates/argo-agent-runner/src/turn_planner.rs` (NEW, conditional) — only if `turn.rs` would breach 500 LOC.
- `crates/argo-agent-runner/src/turn_planner_tests.rs` (NEW, sibling, ≤ 150 LOC) — 4 unit tests: dispatch-without-gate-acts-single-task; gate-true-decomposes-and-records; planner-error-falls-back-to-single; record-error-logs-and-continues.
- `crates/argo-agent-runner/Cargo.toml` — narrowly scoped: add `argo-runtime` dep IF not already present (likely is — verify).

## Files you must NOT touch

- `crates/argo-runtime/src/planner.rs` — planner trait + impls stable; reuse only
- `crates/argo-store/src/subtask.rs` (the SqliteSubtaskSink) — reuse only
- `crates/argo-store/migrations/**` — no schema change (subtask table already exists)
- Any non-runtime/non-agent-runner crate

## Acceptance criteria

- [ ] `DecomposeGate` trait + `HeuristicGate` impl exist in `argo-runtime`
- [ ] Heuristic returns true for explicit list goals (`\n- `, `; `, " and then ")
- [ ] Heuristic returns false for short single-sentence goals
- [ ] Agent dispatch calls `gate.should_decompose` exactly once per goal
- [ ] Gate=true → planner.decompose + record_decomposition called; tracing event emitted
- [ ] Gate=false → no planner call; existing single-task path runs unchanged
- [ ] Planner error → warn + fall back to single task (no crash)
- [ ] record_decomposition error → warn (no crash)
- [ ] 6 unit tests for gate + 4 for dispatch hook
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-runtime -p argo-agent-runner` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-runtime -p argo-agent-runner` passes (skip pre-existing flakes)
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-planner-dispatch-loop-wire`
- PR title: `agent-runner: wire Planner::decompose into dispatch loop via DecomposeGate (mission-planner-dispatch-loop-wire)`

## Notes for the implementer

- The HeuristicGate is intentionally dumb. v1 ships with crude pattern matching; v2 (separate brief) replaces it with an LLM gate. Don't try to be clever here.
- Recursion: if planner.decompose returns children that themselves contain list markers, the existing walker invokes turn-by-turn anyway. Don't add recursion in this brief — let the natural loop handle it.
- The "goal" string source: in the dispatch site, this is the worker's task description / mission statement. Verify the exact source field and cite in PR body.
- AgentDeps already plumbs many primitives — follow the existing pattern. If you need to extend AgentDeps with Planner/Gate/Sink, do so narrowly and document.
- BLOCK and request revision rather than touch `planner.rs` itself.
