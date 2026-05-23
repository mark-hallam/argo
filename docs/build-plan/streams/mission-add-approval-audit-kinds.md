# mission-add-approval-audit-kinds

**Owner**: unclaimed.
**Depends on**: nothing.
**Authored against**: origin/main `56015c0b9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-add-approval-audit-kinds`.

🟡 **P2 — sibling fix-brief unblocking `mission-claim-6-row-4-approval-audit-kinds-pin`.** Adds the missing approval-decide audit-row contract so the row-4 pin test can run.

## Why this brief exists

When `mission-claim-6-row-4-approval-audit-kinds-pin` was claimed (round-24 evaluator handoff), the worker discovered the brief's premise was broken in two ways:

1. **`AuditKind::ApprovalDeclined` does not exist.** Only `ApprovalGranted`, `ApprovalRevoked`, `ApprovalConsumed`, `ApprovalExpiredByTimeout`, `ApprovalAsked`, `ApprovalReplied` are defined in `crates/argo-audit/src/event_kind.rs`. The reject side of an approval reply is currently captured as `ApprovalReplied { reply_kind: "reject" }` (see `crates/argo-audit/tests/claim_6_approval_audit_row_shapes.rs` rounds 19+27 which explicitly note this docs-vs-code drift).

2. **`argo_store::approvals::decide_request` does not emit any audit row.** It performs a SQL `UPDATE approval_request` and returns. No production callsite wraps it with an audit-sink write. The only existing `ApprovalGranted` emissions are in the **tool-grant** flow (`crates/argo-cli/src/handlers/session_approval.rs:75` and `crates/argo-cli/src/handlers/gateway/routes_session_approval.rs:90`) — payload shape `{tool, grant_id, source, note, one_shot, target_glob}`, NOT `{approval_id, decided_by, decision, decided_at, request_summary}`. These are different surfaces.

## What this brief should do

Either:

**Option A — Add the missing variant + wire emission.**
1. Add `AuditKind::ApprovalDeclined` to `crates/argo-audit/src/event_kind.rs` (variant + doc-comment) and its label mapping in `crates/argo-audit/src/event.rs`.
2. Wire an audit-row emission into the canonical approval-decide path with payload shape `{approval_id, decided_by, decision, decided_at, request_summary}` — either inside `decide_request` itself, or at every production callsite (gateway approval-decide route + Slack inbound webhook).
3. Update unit tests in `crates/argo-audit/src/event_tests.rs` for the new label round-trip.

**Option B — Amend the mission-dashboard cell wording.**
If the operator's preference is to keep the audit surface as it stands today (approval-request decides are silent in the audit chain because they're an internal SQL state transition, not a security-relevant event), then the mission-dashboard row text needs to be amended to say "ApprovalReplied { reply_kind }" instead of "ApprovalGranted/ApprovalDeclined", and `mission-claim-6-row-4-approval-audit-kinds-pin` needs to be re-authored against the actual contract.

The operator should decide between A and B before this sibling brief is claimed. **Option B is the smaller-blast-radius choice.** Option A introduces a new audit surface that needs cross-tenant tests, payload-shape stability tests, and chain-verification coverage (the existing `claim_6_approval_audit_row_shapes.rs` already pins those shapes for the kinds that DO exist).

## Files you may touch

If pursuing Option A:
- `crates/argo-audit/src/event_kind.rs` — add `ApprovalDeclined` variant + doc comment.
- `crates/argo-audit/src/event.rs` — add label mapping for `"approval.declined"`.
- `crates/argo-audit/src/event_tests.rs` — assert label round-trip for the new variant.
- `crates/argo-store/src/approvals.rs` — wire audit emission into `decide_request` (or at callsites).
- `crates/argo-audit/tests/claim_6_approval_audit_kinds.rs` — the original row-4 pin test (re-authored as part of this brief OR left for the now-blocked parent brief to land).

If pursuing Option B:
- `docs/skycot/evals/scheduled/2026-05-23/claim-dashboards.md` — re-word row 4 to reflect the actual contract.
- `docs/build-plan/streams/mission-claim-6-row-4-approval-audit-kinds-pin.md` — re-author against the actual contract OR mark superseded.

## Files you must NOT touch

- The existing `crates/argo-audit/tests/claim_6_approval_audit_row_shapes.rs` — already pins the actual contract for the kinds that exist. Do not duplicate.
- Production callsites for `ApprovalGranted` in `session_approval.rs` / `routes_session_approval.rs` — those serve the tool-grant flow, not the approval-request-decide flow.

## Acceptance criteria

- [ ] Operator chose Option A vs Option B in PR description.
- [ ] If Option A: `cargo test -p argo-audit` + `cargo test -p argo-store` green, including the row-4 pin test.
- [ ] If Option B: dashboard wording amended, parent brief either re-authored or marked SUPERSEDED.
- [ ] BUILD_STATUS.md row updated.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Adding emission into the gateway approval-decide route (deferred to a follow-up wire-up brief if Option A is chosen).
- Changing the audit-row chain hash semantics.

~50-300 LOC depending on chosen option.
