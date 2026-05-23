# mission-claim-6-row-4-approval-audit-kinds-pin

**Owner**: claude.
**Depends on**: nothing (additive test only).
**Authored against**: origin/main `bdf57f458` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-6-row-4-approval-audit-kinds-pin`.

🟡 **P2 — closes out Claim 6 MVP to ~100%.** Source: evaluator round-24 SHARED.md handoff (cross-tenant Slack approval queue isolation FLIPPED 🟢 → 🟢🟢) — only row 4 (Approval audit kinds: `ApprovalGranted` / `ApprovalDeclined` audit-row shape) remains 🟢 substrate-only without a live regression contract.

> "Round 24 — Claim 6 MVP row 5 e2e-verified. Score: ~85% → ~95% — only row 4 (Approval audit kinds: ApprovalGranted/ApprovalDeclined audit-row shape) remains 🟢 substrate-only."

Pin the contract: when an operator approves or declines an approval-request via the Slack inbound path (or any approval-decide path), the resulting audit row MUST have `AuditKind::ApprovalGranted` (or `::ApprovalDeclined`) with a payload shape that includes (`approval_id`, `decided_by`, `decision`, `decided_at`, `request_summary`).

## Scope

Add a new integration test `crates/argo-audit/tests/claim_6_approval_audit_kinds.rs` (~200-300 LOC) that:

1. Seeds an approval-request via the existing `NewApprovalRequest` helper (Slack-routed, tenant_a).
2. Calls the production `decide_request` (or whatever the canonical decide-entry is) with `ApprovalDecision::Approve` + a `decided_by` operator identity.
3. Asserts EXACTLY ONE new audit row appears with `kind == AuditKind::ApprovalGranted` (compile-asserts the variant exists too — if it doesn't, the brief blocks on a sibling brief to add it).
4. Asserts the audit row's `payload_json` decodes to a struct with named fields: `approval_id` (= the seeded id), `decided_by` (= operator identity), `decision` (= "Approve"), `decided_at` (ISO-8601 timestamp), `request_summary` (= the seeded approval body's first 80 chars).
5. Repeat with `ApprovalDecision::Reject` → assert audit row's kind == `AuditKind::ApprovalDeclined` + same payload shape with decision == "Reject".
6. Cross-tenant negative: tenant_a decides; tenant_b queries audit feed → returns zero rows containing the approval_id.

If `AuditKind::ApprovalGranted` / `::ApprovalDeclined` do NOT exist in `crates/argo-audit/src/`:
- Surface that in the PR body as a P0 audit gap.
- Author a sibling fix-brief `mission-add-approval-audit-kinds` BEFORE this test can pass.
- Mark this brief BLOCKED on the sibling, exit cleanly.

If they DO exist, the test runs.

## Files you may touch

- `crates/argo-audit/tests/claim_6_approval_audit_kinds.rs` — NEW.
- `docs/skycot/evals/scheduled/2026-05-23/claim-dashboards.md` — flip row 4 of Claim 6 MVP from 🟢 substrate-only to 🟢🟢 e2e-verified.
- `docs/build-plan/streams/mission-add-approval-audit-kinds.md` — NEW sibling brief IF audit variants don't exist.

## Files you must NOT touch

- Production source in `crates/argo-audit/src/` — additive test only.
- `crates/argo-channels/` — out of scope; the decide-entry path uses existing channel inbound code.
- Gateway routes.

## Acceptance criteria

- [ ] New test compiles + runs green on origin/main HEAD.
- [ ] Test seeds approval → decides Approve → asserts ApprovalGranted audit row with the 5 named payload fields.
- [ ] Test seeds approval → decides Reject → asserts ApprovalDeclined audit row.
- [ ] Cross-tenant negative passes (tenant_b sees zero of tenant_a's decision rows).
- [ ] If audit variants don't exist, sibling fix-brief is authored same PR + this brief is marked BLOCKED with the sibling brief-id in the BUILD_STATUS row.
- [ ] `cargo clippy -p argo-audit --tests -- -D warnings` clean.
- [ ] `cargo test -p argo-audit claim_6_approval_audit_kinds` passes.
- [ ] BUILD_STATUS.md updated with PR number.
- [ ] Claim-dashboards.md row flipped.

## Test plan

- Crate-scoped cargo test.
- No real Slack network — the inbound test should use the existing test-helper that simulates a decoded approval-decision (same pattern as round-23 `claim_6_slack_inbound.rs`).

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Adding new audit fields beyond the 5 named.
- Changing the audit-feed query surface.
- The Slack-inbound transport itself (already e2e-verified round-23).

~250-400 LOC. Closes Claim 6 MVP scorecard to ~100% — the last substrate-only cell.
