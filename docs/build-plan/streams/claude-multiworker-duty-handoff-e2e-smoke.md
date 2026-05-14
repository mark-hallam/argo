# claude-multiworker-duty-handoff-e2e-smoke — E2E smoke for multi-worker handoff with shared memory (scenario #3)

**Owner**: claude. **Depends on**: nothing (per-worker memory scope is queued but workspace-shared sufficient for v1 demo).
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-multiworker-duty-handoff-e2e-smoke`.

## Scope

Phase C audit found that MISSION.md success scenario #3 ("Multi-worker handoff works end-to-end. Worker A completes its duty, hands off to Worker B with shared memory; B continues without re-asking the operator the same questions A asked") is ADEQUATE-tested at the unit/integration level (`argo-orchestrator/tests/handoff_audit_smoke.rs` proves the audit row) but **no E2E test proves Worker B actually uses the shared memory in its turn**.

This brief adds the missing assertion: Worker A runs → completes → hands off → Worker B runs → asserts B's prompt includes context A produced.

LOC: ~200.

## Files you may touch

- NEW `crates/argo-orchestrator/tests/multiworker_handoff_e2e_smoke.rs` — integration test. Workers A + B share a workspace memory namespace; A writes a fixture memory entry; B reads it during its turn; assertion: B's rendered system prompt contains the memory snippet.
- NEW `crates/argo-orchestrator/tests/fixtures/multiworker_handoff/` — fixture vertical + duty configs for both workers.
- `scripts/smoke-skycot.sh` — narrowly: invoke as conditional step 22. ≤15 modified lines.

## Files you must NOT touch

- Production code for the handoff path (this is a TEST brief; consume existing APIs).
- argo-memory production code.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `cargo test --test multiworker_handoff_e2e_smoke -p argo-orchestrator` passes.
- [ ] Test is deterministic (no LLM calls; mock memory writes; mocked decisions).
- [ ] Assertion: Worker B's rendered system prompt contains the memory entry Worker A wrote.
- [ ] Negative case: a worker in a DIFFERENT workspace (or different tenant) does NOT see the memory.
- [ ] `bash scripts/smoke-skycot.sh` includes step 22 + still exits 0.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-multiworker-duty-handoff-e2e-smoke`
- PR title: `test(orchestrator): multi-worker handoff E2E with shared memory (claude-multiworker-duty-handoff-e2e-smoke)`

## Notes for the implementer

- Mission-claim alignment: scenario #3 (multi-worker handoff) + claim 1 (teammate persistence across worker boundaries).
- Workspace-shared memory is sufficient for v1; per-worker memory is queued (`mission-per-worker-memory-scope` or equivalent — verify at claim time).
- The negative test (cross-tenant isolation) is critical. Skipping it leaves a real-world leak risk.
- Drift-prevention: brief adds NEW test files only; no new pub types.
