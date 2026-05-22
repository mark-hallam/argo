# mission-workflow-audit-write-transactional

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `6e5ab8d5b` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-workflow-audit-write-transactional`.

🟡 **P1 MISSION-CLAIM-6 — workflow audit write is non-transactional.** Source: evaluator round-7 SHARED.md handoff 2026-05-23T06:30:00Z.

> "🟡 mission-workflow-audit-write-transactional — audit write is OUTSIDE workflow write tx, silent gap if it fails (mission-claim-6, P1)"

The workflow handlers (`set_head` / `archive` / `rollback`) correctly use transactions for the workflow row writes. BUT the corresponding `audit_record` row is written OUTSIDE the transaction — if the audit write fails (disk full, lock contention, etc.) the workflow change still persists, breaking the "every action audited" invariant of mission claim 6.

This is the same shape as the audit-export tenant-blind bug closed via #2051 (round 3) — silent data integrity gap, only visible under failure.

## Scope

For each of the workflow handlers that write audit:

1. **Identify the current pattern** — find via `rg "set_head|archive|rollback" crates/argo-cli/src/handlers/gateway/routes_workflow*.rs` + `rg "AuditSink::write|emit_audit" crates/argo-cli/src/handlers/gateway/routes_workflow*.rs`.
2. **Move the audit write INSIDE the transaction** — both the workflow row mutation AND the `audit_record` insert must commit together OR rollback together.
3. **If transaction failure happens before audit**, no row should be persisted. If transaction failure happens AFTER audit insert (rare — both in same tx), rollback discards both.

### Implementation approach

Two options depending on the existing seam:

**Option A (preferred if `WorkflowStore::set_head` already takes an `&mut Transaction`)**: pass the audit-row-builder into the store method; method writes both inside its own tx.

**Option B (if the store method commits internally)**: refactor to expose a `set_head_in_tx(tx, ...)` variant that does NOT commit, then the handler owns the tx, calls `set_head_in_tx` + `audit_insert_in_tx`, then commits. This is the cleaner long-term shape.

Pick B if the refactor is contained (<200 LOC); otherwise A.

### Regression tests

For each affected handler:
1. **Happy path tx**: workflow write + audit row both land.
2. **Failure injection**: mock the audit insert to fail mid-tx → assert workflow row did NOT persist (rolled back).

The failure-injection test is load-bearing — without it, the contract isn't pinned.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` and siblings (find via grep for the 3 mutators).
- `crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs` — add failure-injection test.
- `crates/argo-store/src/workflow_def.rs` (or wherever the store layer lives) — narrowly scoped: add `_in_tx` variant if going Option B.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `argo-audit` crate internals — the audit write itself is fine, only its transaction membership changes.
- Unrelated workflow handlers (CRUD that doesn't write audit).
- Other crate's transaction patterns.

## Acceptance criteria

- [ ] All 3 workflow audit-emitting handlers wrap audit + workflow in single tx.
- [ ] Failure-injection regression test for each handler asserts atomic rollback.
- [ ] `cargo clippy -p argo-cli -p argo-store --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-workflow-audit-write-transactional`
- PR title: `fix(gateway): workflow audit writes share tx with workflow mutation (mission-workflow-audit-write-transactional)`

## Notes for the implementer

- ~250-400 LOC expected. Handler edits ~50, store-layer split ~80, 3 tests ~150, harness for failure injection ~50.
- **Failure-injection is the contract**. Without it, this is "tighten the code" not "prove the invariant."
- **Don't reduce audit verbosity** — the audit row is what it is. This brief is about transaction membership only.
- This is also relevant to the `claude-conductor-fix-by-enforcement` brief shipped this tick — audit-row reliability is foundational.
- Cite evaluator round-7 verbatim.
