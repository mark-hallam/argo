# n-of-m-multi-step-approval (FINAL SPRINT · v1.1 governance) — threshold (N-of-M) approvals

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh n-of-m-multi-step-approval`.

## Why (evidence)
Approvals today are **single-approver**: one operator reply settles the action. The
gate evaluator (`crates/argo-identity/src/auto_approval.rs:26` `PendingAction`,
`types.rs:113` `AutoApprovalPolicy`) and the settle path
(`crates/argo-cli/src/handlers/gateway/routes_channels.rs:85` `settle` →
`Decision::Approve`/`Reject`) have NO threshold / multi-approver concept (`rg
'threshold|approvers|quorum|n_of_m|vote'` over `argo-identity` returns only the
unrelated circuit-breaker threshold). The approval store is `approval_request`
(migration `crates/argo-store/src/migrations/v44.rs`), and there's a server-defined
**bundle** precedent (`v68` `approval_bundle` + `approval_bundle_member`) to mirror for
shape.

This brief adds **N-of-M threshold approvals** (v1.1 governance): an approval payload
may declare `approvers:[...]` + `threshold:N`; each approver's decision is recorded
separately (a vote ledger); the action stays HELD until N approvals are recorded; every
vote is audited.

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'pub struct PendingAction|pub struct AutoApprovalPolicy' crates/argo-identity/src` — EXTEND additively (an optional `threshold`/`approvers`); a missing field = today's single-approver behavior (back-compat).
- `rg -n 'pub(super) async fn settle|enum Decision|reject_action' crates/argo-cli/src/handlers/gateway/routes_channels.rs` — the settle path must record a VOTE and only finalize when the threshold is met; do NOT fork a parallel settle.
- `rg -n 'CREATE TABLE approval_request|approval_bundle|approval_bundle_member' crates/argo-store/src/migrations/v44.rs crates/argo-store/src/migrations/v68.rs` — mirror the bundle-member shape for the vote ledger; next free migration slot; register in the registry; additive only.
- `rg -n 'pub enum AuditKind' crates/argo-audit/src/event_kind.rs` — add ONE additive audit kind for an individual vote (distinct from `ApprovalGranted`/`ApprovalDeclined`, which remain the terminal settle events); add its string mapping in `event.rs` + a test row.
- `rg -n 'approval_token|TokenClaims|verify_token' crates/argo-cli/src/handlers/gateway/routes_channels.rs` — each approver's signed token identifies WHICH approver voted (so one approver cannot vote twice).

## Files you may touch
- new `crates/argo-store/src/migrations/vNN.rs` (additive: an `approval_vote` table —
  `approval_request_id`, `approver`, `decision`, `voted_at`, tenant_id; UNIQUE on
  `(approval_request_id, approver)` so a vote is idempotent per approver). Register in
  the registry (`scripts/check-migration-slots.sh` must stay green).
- `crates/argo-identity/src/types.rs` + `auto_approval.rs` (additive optional
  `threshold` + `approvers` on the policy/pending shape; absence = single-approver).
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs` (and/or a sibling
  `routes_approvals_threshold.rs`) — settle records a vote; finalizes
  (`ApprovalGranted`) only once `>= threshold` distinct approvals exist; a `Reject`
  vote settles per the policy (e.g. any reject denies, or configurable — document the
  chosen rule); audit every vote. Keep each file < 500 LOC.
- `crates/argo-audit/src/event_kind.rs` + `event.rs` (the new per-vote audit kind).
- tests siblings.

## Acceptance (truth-bounded + testable)
- An integration test: an approval with `threshold:2`, `approvers:[a,b,c]` — one
  approve vote leaves the action HELD (not granted); a second distinct approve vote
  crosses the threshold and finalizes (`ApprovalGranted`); a per-vote audit row exists
  for each.
- A test: the SAME approver voting twice is idempotent (UNIQUE constraint) and does NOT
  count as 2 toward the threshold.
- A back-compat test: an approval with NO `threshold`/`approvers` behaves exactly like
  today's single-approver settle (one approve → granted).
- A **tenant-isolation test**: votes for tenant A's approval are never visible to /
  countable by tenant B.
- A reject-rule test pinning the documented denial semantics.
- `cargo test -p argo-cli -p argo-identity` green; `scripts/check-migration-slots.sh`
  green.

## On ship
Add/flip the FEATURE_MATRIX governance row (single-approver → N-of-M threshold
approvals, v1.1) with the commit SHA + route + migration in the SAME commit, then
`python3 scripts/render-feature-matrix.py`.

## Effort
M (Rust). ~300–450 LOC incl. tests. Additive (one additive migration + additive
policy fields + additive audit kind); single-approver path unchanged when the new
fields are absent.
