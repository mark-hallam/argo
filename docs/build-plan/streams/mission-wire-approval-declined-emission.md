# mission-wire-approval-declined-emission

**Owner**: unclaimed.
**Depends on**: `mission-add-approval-audit-kinds` (MERGED #2163 — `AuditKind::ApprovalDeclined` exists on origin/main).
**Authored against**: origin/main `952e42591` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-wire-approval-declined-emission`.

🟡 **P2 — follow-up wire-up brief.** #2163 added `AuditKind::ApprovalDeclined` as a first-class variant but explicitly deferred production emission (argo-store cannot cycle-depend on argo-audit; zero callers of `decide_request`; gateway approve route uses `worker_actions_transitions` flow not `decide_request`). This brief wires the missing emission at the canonical composition site so the dashboard row-4 contract is end-to-end live, not just type-level.

## Why this brief exists

Per #2163 PR body and `crates/argo-audit/tests/claim_6_approval_audit_kinds.rs` docstring:

> Production emission of `ApprovalDeclined` is deferred: the canonical decide path lives at `argo-store::approvals::decide_request`, but `argo-store` cannot depend on `argo-audit` (the dependency already runs the other way) and the gateway approve route currently uses the `worker_actions_transitions` flow, not `decide_request`. The follow-up wire-up brief will route emission through whichever caller composes the store + audit-sink dependencies (gateway `routes_approvals_json::decide_response` is the leading candidate).

That follow-up brief is this one.

## What this brief should do

Wire `ApprovalDeclined` emission at the gateway `decide_response` composition site (`crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs:187`). This is the production reject path: when an operator POSTs `decide: "refuse"`, `worker_actions_transitions::cancel()` runs the state transition; this brief inserts an `AuditSink::write` for `AuditKind::ApprovalDeclined` immediately after that succeeds.

Mirror the existing `ApprovalGranted` emission pattern from `crates/argo-cli/src/handlers/session_approval.rs:75` for the approve side. The reject-side payload shape:

```json
{
  "approval_id": "<uuid>",
  "decided_by": "<principal>",
  "decision": "refuse",
  "decided_at": "<rfc3339 utc>",
  "request_summary": "<one-line summary of what was refused>"
}
```

Acceptance gate: row-4 dashboard cell flips from 🟡 substrate-only to 🟢🟢 e2e-verified once this brief lands + a regression test pins the byte-shape of the row through the full gateway POST.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` — wire `AuditSink::write` for `ApprovalDeclined` after `worker_actions_transitions::cancel()` succeeds on the refuse branch (~line 187).
- `crates/argo-cli/src/handlers/gateway/routes_approvals_json_tests.rs` (or new test file if absent) — pin payload shape end-to-end via `axum::test`.
- `crates/argo-cli/tests/claim_6_approval_declined_e2e.rs` (new file) — sibling to `claim_6_approval_audit_row_shapes.rs`. Pin the full POST → decide → row-emit loop.

## Files you must NOT touch

- `crates/argo-audit/src/event_kind.rs` — already shipped via #2163.
- `crates/argo-store/src/approvals.rs::decide_request` — the cycle prevents emission here.
- The existing `ApprovalGranted` callsite in `session_approval.rs:75` — that serves the tool-grant flow, not the approval-decide flow.

## Acceptance criteria

- [ ] `ApprovalDeclined` audit row emits on `decide: "refuse"` POST to gateway.
- [ ] Payload shape matches the brief above (5 fields, tenant_id stamped both in payload + denorm column).
- [ ] `cargo test -p argo-cli --test claim_6_approval_declined_e2e` green.
- [ ] Existing 13/13 gateway approval tests stay green.
- [ ] No row emitted on `decide: "approve"` (the approve side stays on the existing tool-grant flow until a separate brief wires it).
- [ ] Tenant-isolation: cross-tenant attempt does NOT leak rows across tenants.
- [ ] BUILD_STATUS.md row updated to MERGED.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## LOC budget

~120-180 LOC total (one production call + one e2e test file + one-row test-fixture extension).
