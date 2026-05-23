# eval-orchestrator-cross-tenant-handoff-negative-test

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-orchestrator-cross-tenant-handoff-negative-test`.

🟡 **P1 MISSION-CLAIM-2 — orchestrator substrate scoped but no regression test pins it.** Source: evaluator round-12 SHARED.md handoff 2026-05-23T12:00:00Z.

> "argo-orchestrator audited (9127 LOC, ~30 files, multi-worker session + handoff + escalation + duty_scheduler + auction). SUBSTRATE TENANT-SCOPED at SQL level — every `persist_handoffs.rs` query binds tenant_id... 🟡 NEW P1 GAP: ZERO cross-tenant negative tests in the entire crate — `rg 'cross.tenant|tenant_a|tenant_b'` returns zero matches in tests. SQL is right but no regression test pins it; same shape as the round-3 audit-export pattern that shipped a bug."

The orchestrator's SQL is correct today. But the same shape — "SQL looks tenant-scoped, no test pins it" — is exactly what shipped the audit-export P0 bug (#2046 → #2051 fix-by-pair). Without a regression test, a future refactor can silently drop the predicate and ship.

## Scope

Add cross-tenant negative tests for every public orchestrator surface. The brief is read-only on production code — if you find an actual gap, ship `#[ignore]`'d + FINDINGS-ONLY note.

### Surfaces to test

For each, write a paired test: bootstrap tenant_a + tenant_b each with a worker + a session/handoff; call from tenant_b's principal → assert tenant_a's row is NOT visible.

1. **`Handoff::find` / `insert_handoff` / `list_pending`** in `persist_handoffs.rs` — test: tenant_a creates handoff for tenant_a's worker; tenant_b's `list_pending(tenant_b)` returns empty; tenant_b's `find_handoff(handoff_id)` returns Err(NotFound) NOT the leaked row.
2. **`ack` / `settle`** — same shape: tenant_b tries to ack tenant_a's handoff → assert refusal.
3. **`retry.rs::record_attempt` / `list_failed`** — cross-tenant read returns empty.
4. **`duty_scheduler_store.rs::list_due` / `mark_fired`** — tenant_b's list_due doesn't include tenant_a's duties.
5. **`sla.rs::register_deadline` / `breached_for_tenant`** — tenant_b's breached doesn't include tenant_a's.

For each: assert byte-identical response shape between cross-tenant (forbidden) and nonexistent-id (genuine 404) — per the worker-handler existence-leak pattern from #2068.

### Where the tests live

`crates/argo-orchestrator/tests/cross_tenant_handoff_test.rs` (or per-surface sibling files if 500-LOC ceiling forces a split).

### Audit emission

For each cross-tenant attempt, the orchestrator should emit an audit row (`HandoffCrossTenantAttempt` or equivalent). If no such audit kind exists, file a follow-up brief (`mission-orchestrator-cross-tenant-audit-kind`); don't add the variant in this brief (out of read-only audit scope).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: crates/argo-orchestrator/tests/cross_tenant_handoff_test.rs` (~250-400 LOC). Split into siblings if hitting 500 ceiling.
- `crates/argo-orchestrator/Cargo.toml` — narrowly scoped: dev-deps for test fixtures.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — update argo-orchestrator row 🟢.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-orchestrator/src/**` — strict read-only. This is an audit, NOT a refactor. If you find a real cross-tenant leak, ship `#[ignore]`'d FINDINGS-ONLY + name the fix-brief.
- `argo-audit` crate — out of scope.
- Other crates outside the audit chain.

## Acceptance criteria

- [ ] Cross-tenant negative tests for all 5 surfaces above.
- [ ] Each test asserts byte-identical NotFound response between cross-tenant + genuine-nonexistent.
- [ ] If any surface actually leaks → ship `#[ignore]`'d + FINDINGS-ONLY + fix-brief recommendation.
- [ ] Gap manifest argo-orchestrator row 🟡 → 🟢 (or kept 🟡 with note if findings).
- [ ] `cargo test -p argo-orchestrator --test cross_tenant_handoff_test` passes.
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `eval-orchestrator-cross-tenant-handoff-negative-test`
- PR title: `test(orchestrator): cross-tenant handoff/duty/retry/sla negatives (eval-orchestrator-cross-tenant-handoff-negative-test)`

## Notes for the implementer

- ~250-400 LOC expected. 5 surfaces × ~50-80 LOC each.
- **The byte-identical assertion is load-bearing** — per #2068's pattern, just "returns 404" is too weak; the cross-tenant branch must be indistinguishable from the genuine-nonexistent branch.
- **Cite the audit-export precedent**: round-3 found the same shape (SQL correct, no test) and a regression later shipped. This brief locks the contract before the regression can happen.
- Round-12 evaluator handoff is the source; cite verbatim in PR body.
