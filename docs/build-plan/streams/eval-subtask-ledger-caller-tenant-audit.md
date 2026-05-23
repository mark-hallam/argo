# eval-subtask-ledger-caller-tenant-audit

**Owner**: claude (evaluator-shape brief — investigation + per-caller findings + tests).
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-subtask-ledger-caller-tenant-audit`.

🟡 **P1 — subtask_ledger keys by id alone; caller-side tenant audit needed.** Source: evaluator round-13 SHARED.md handoff 2026-05-23T12:30:00Z.

> "🟡 subtask_ledger: all 12 queries key by id alone — same class as round-3 audit_detail bug fixed in #1992; need caller-side audit."

The `subtask_ledger` SQL queries all use `WHERE id = ?` (bare UUID lookup) without binding tenant_id. This is the EXACT shape that caused the audit_detail cross-tenant leak (round-3 → fixed in #1992): "lookup-by-id is fine IF the caller never passes an id from a different tenant, but no test pins that contract." Audit every caller; ship tests.

## Scope

Investigation brief: walk every callsite of `subtask_ledger` queries (find via `rg 'subtask_ledger|SubtaskLedger' crates/`), verify each caller already has tenant context + uses tenant-scoped resolution to obtain the id. For any callsite that takes an id from a request/HTTP param without tenant-checking, ship a cross-tenant negative test + FINDINGS-ONLY note.

### Deliverable

Report at `docs/skycot/evals/scheduled/2026-05-23/eval-subtask-ledger-caller-tenant-audit.md`:
- Caller inventory (file:line for each `subtask_ledger` query callsite)
- Per-caller verdict: 🟢 tenant-checked / 🟡 needs cross-tenant test / 🔴 actual leak
- For 🔴 findings: name the fix-brief

### Optional regression tests

For each callsite without coverage: `crates/argo-cli/tests/subtask_ledger_cross_tenant_<caller>_test.rs` — cross-tenant negative pinning the contract.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: docs/skycot/evals/scheduled/2026-05-23/eval-subtask-ledger-caller-tenant-audit.md` — report.
- `NEW: crates/<caller-crate>/tests/subtask_ledger_cross_tenant_*_test.rs` — per-caller tests if needed.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — flip subtask_ledger row.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `subtask_ledger` query source — read-only audit. If real leak found, ship `#[ignore]`'d + FINDINGS-ONLY note + fix-brief recommendation.

## Acceptance criteria

- [ ] Caller inventory complete (all 12 query callsites mapped).
- [ ] Per-caller verdict per the 🟢/🟡/🔴 rubric.
- [ ] Cross-tenant tests added for any 🟡 callsite (or `#[ignore]`'d + FINDINGS-ONLY if 🔴).
- [ ] Gap manifest updated.
- [ ] Cost target 30-60K tokens.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `eval-subtask-ledger-caller-tenant-audit`
- PR title: `eval(subtask-ledger): caller-side tenant audit + per-caller tests (eval-subtask-ledger-caller-tenant-audit)`

## Notes for the implementer

- ~150-300 LOC. Mostly report; tests as discovered.
- The audit_detail bug (#1992) is the precedent — same shape, real leak shipped. Don't trust "the caller has tenant context" without a test.
- Cite round-13 verbatim.
