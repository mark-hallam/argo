# mission-claim-4-mvp-row-6-gateway-promote-audit-pin

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `d3929941e` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-4-mvp-row-6-gateway-promote-audit-pin`.

🟡 **P2 — Claim 4 MVP row 6 audit-emit pin (currently theater-grade).** Source: evaluator round-30 SHARED.md handoff 2026-05-23T22:30:00Z.

> "gateway-side promote in `routes_worker_skills.rs:326` is WEAKLY pinned (`routes_worker_skills_tests.rs:396-403` only assert `COUNT(*) WHERE kind='skill.candidate.promoted' >= 1`). A refactor dropping `payload[skill_id]`, inverting `promotion_kind`, dropping `AuditSink::new_with_tenant` tenant-stamping, or leaking cross-tenant rows would NOT flip the existing test red. 13/13 gateway promote tests pass cold but the audit-emit assertion is theater-grade."

## Scope

A 4-test file pinning the `POST /api/v1/workers/<id>/skills/<sid>/promote` audit-emit contract beyond `COUNT(*) >= 1`.

### Test 1 — payload shape pin
On a successful promote, the emitted audit row's `payload_json` must carry: `worker_id`, `skill_id`, `vertical_id`, `promotion_kind`, `source` (`gateway` or equivalent), `promoted_by` (caller principal). Assert each field byte-exact against an expected JSON object — not just `is_object()`.

### Test 2 — denorm tenant_id stamping
The denorm `tenant_id` column on the audit row must match the caller's tenant. A foreign tenant_id (smuggled via header) is refused upstream by RBAC; this test focuses on the happy path stamping correctness.

### Test 3 — cross-tenant isolation
Promote a candidate as tenant_a; tenant_b's `SELECT FROM audit_record WHERE kind = 'skill.candidate.promoted' AND tenant_id = tenant_b` returns ZERO rows. Tenant_a's row is invisible across the tenant boundary.

### Test 4 — atomic ratchet-and-audit
If the trust ratchet succeeds but the audit-write fails (e.g. inject a faulty `AuditSink`), the ratchet must roll back — no half-promoted state. Use a `FaultyAuditSink` injectable that returns an error on write.

## Acceptance

- New file: `crates/argo-gateway/tests/claim_4_mvp_row_6_gateway_promote_audit_pin.rs` (≤200 LOC).
- 4 tests green on `cargo test -p argo-gateway`.
- Crate-scoped clippy clean.
- No production code changes — pure regression-test pin.
- May-touch: `crates/argo-gateway/tests/`, `crates/argo-gateway/src/routes_worker_skills.rs` (read-only, for understanding the seam).

## Parallelism
None. Single-claim, single-machine.

## Done when
PR opens, tests green, conductor admin-merges.
