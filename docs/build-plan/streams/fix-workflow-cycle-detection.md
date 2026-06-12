# fix-workflow-cycle-detection

**Owner**: claude or codex — DEC-006-adjacent (Rust); conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `de0e01ea3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-workflow-cycle-detection

## Problem (discovery-verified 2026-06-12)

The TOML workflow DAG validator has **no cycle detection**:
`crates/argo-vertical/src/workflow.rs` `validate_workflow` (~:134-165)
checks only that each `depends_on` name resolves to an existing step — a
workflow with a dependency cycle (A→B→A) passes validation and would
deadlock/loop at execution. Three `#[ignore]`'d tests in
`crates/argo-vertical/src/workflow_edge_tests.rs` are waiting for the
implementation (confirm the exact file + test names at claim time — the
discovery agent cited workflow_edge_tests.rs; grep `#[ignore]` +
`cycle` in argo-vertical to pin them).

## Fix

Add a cycle-detection pass to `validate_workflow` (DFS / Kahn's
topological-sort over the `depends_on` edges; on a back-edge, return a
`LoadError`/validation error naming the cycle path). Self-dependency
(A→A) and multi-node cycles both rejected. Un-ignore the 3 pinned tests.
Pure validation — no execution-path change.

## May touch

- `crates/argo-vertical/src/workflow.rs` (+ the workflow_edge_tests it
  un-ignores)

## Acceptance / verifier

1. The 3 `#[ignore]`'d cycle tests un-ignored + GREEN; a self-cycle and a
   2-node cycle are both rejected with a clear error; a valid DAG still
   passes.
2. `cargo test -p argo-vertical` green; clippy clean; no change to valid
   workflow loading.

~80-120 LOC. **🟡 P2 — removes 3 ignored tests; closes a real DAG-validity
gap.**
