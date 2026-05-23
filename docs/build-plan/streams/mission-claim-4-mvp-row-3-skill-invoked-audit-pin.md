# mission-claim-4-mvp-row-3-skill-invoked-audit-pin

**Owner**: unclaimed.
**Depends on**: nothing (row-3 substrate exists; this pins the byte-shape).
**Authored against**: origin/main `952e42591` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-4-mvp-row-3-skill-invoked-audit-pin`.

🟡 **P2 — Claim 4 MVP row 3 (SkillInvoked audit payload shape) substrate-thin → 🟢🟢 e2e pin.** Per evaluator round 31c (SHARED.md 2026-05-23T23:30Z), Claim 4 MVP coverage is now ~95% with 4 of 7 rows 🟢🟢 e2e-verified (rows 4+5+6+7); rows 1+3 remain 🟡 substrate-thin. This brief closes row 3 — the byte-shape pin for the `skill.invoked` audit row.

## Why this brief exists

Claim 4 MVP row 3 from `docs/skycot/evals/scheduled/2026-05-23/claim-dashboards.md`:

> Row 3: SkillInvoked audit row carries skill_id + version + outcome + worker_id + tenant_id; chain-verifies clean.

Today this is asserted at the unit-test level in `crates/argo-audit/src/event_tests.rs` (label round-trip only) and at the production-emission level inside `agent-runner` (the runtime emits the row when a skill_invoke step completes). The row is NOT pinned by a regression test that:
- Reads back the emitted row through `argo audit show` / `read_session` SQL
- Verifies the full payload shape (skill_id + version + outcome + worker_id + tenant_id)
- Verifies the row chain-verifies clean across a 3-row sequence (so any future refactor that drops a payload field would flip the test red)
- Verifies cross-tenant isolation (tenant_b can't see tenant_a's skill.invoked row)

This brief lands that regression test alongside the existing claim-4 pins (rounds 28/29/31b/31c).

## What this brief should do

Author `crates/argo-skill/tests/claim_4_mvp_row_3_skill_invoked_audit_pin.rs` (or `crates/argo-audit/tests/claim_4_mvp_row_3_skill_invoked_audit_pin.rs` if the natural test home is the audit crate). 4 tokio tests:

1. **Row shape**: drive a skill_invoke through `argo-agent-runner` end-to-end; assert the emitted audit row has `kind == AuditKind::SkillInvoked`, payload carries all 5 declared fields (skill_id, version, outcome, worker_id, tenant_id), denorm `tenant_id` column matches `payload.tenant_id`.
2. **Outcome=success vs outcome=failure**: both branches emit rows; failure path includes error context in payload.
3. **Chain verification**: 3-row sequence (skill.invoked → skill.invoked → skill.invoked); `verify_session` returns ok=true rows=3 breaks=[].
4. **Cross-tenant isolation**: tenant_a invokes a skill → tenant_b's `list_audit` returns ZERO rows; tenant_a sees its own.

Mirror the test shape used by `claim_6_approval_audit_row_shapes.rs` and `claim_4_mvp_row_4_novel_sequence_detection.rs`.

## Files you may touch

- `crates/argo-skill/tests/claim_4_mvp_row_3_skill_invoked_audit_pin.rs` (new test file, ~150-200 LOC).
- If the test home is more naturally argo-audit (no agent-runner integration needed because audit can be driven directly via `AuditSink::write`): `crates/argo-audit/tests/claim_4_mvp_row_3_skill_invoked_audit_pin.rs` instead.
- Brief should pick whichever home matches the existing `claim_4_mvp_row_*` test files for consistency.

## Files you must NOT touch

- Production agent-runner code — this is a pin, not a wire-up brief. If the test reveals a missing field in the emitted payload, file a separate fix brief; do NOT silently patch agent-runner here.
- Existing `claim_6_approval_audit_row_shapes.rs` — separate row contract.

## Acceptance criteria

- [ ] 4 tokio tests landed, all green on origin/main HEAD at claim time.
- [ ] Cargo test runs in <30s cold.
- [ ] Tests fail meaningfully if any of the 5 declared payload fields is dropped (negative-test mindset: simulate a refactor that drops `version` and confirm the test catches it via inverted assert in the test docstring).
- [ ] Chain-verification asserts ok=true rows=3 breaks=[] on the multi-row case.
- [ ] BUILD_STATUS.md row updated.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## LOC budget

~150-220 LOC (one test file). Below the 500 LOC per-file cap.
