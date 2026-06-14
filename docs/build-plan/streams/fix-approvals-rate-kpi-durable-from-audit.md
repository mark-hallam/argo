# fix-approvals-rate-kpi-durable-from-audit

**Authored against:** origin/main `de775470c` on 2026-06-14
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-approvals-rate-kpi-durable-from-audit`

## Problem (2026-06-14 25x-soak finding #2, server-confirmed across ~10 workspaces)

The `/approvals` **APPROVAL RATE** KPI resets to "—" on reload after the operator
approves holds. Root cause: `computeApprovalRate(items)`
(`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:73`) derives the rate
from the **current pending list** — it does `items.filter(i => i.decision != null)`.
But a resolved hold/approval DROPS OUT of the `/approvals` list (the probation
read-side filters resolved `(session_id, tool)` keys; memory/seeded arms likewise
surface only pending). So on any fresh load `decided.length === 0` → returns `null`
→ the KPI shows "—", even after the operator has approved many actions. The metric
is structurally unable to reflect history because its data source is the **ephemeral
pending queue**, not the durable decision history.

This is NOT the probation re-hold growth — that is DEC-025 #2 by-design plus the
#2936 convergence fix (restored 2026-06-14). This brief is ONLY about the rate
metric's data source.

## Fix (read-side over the durable audit chain — NO parallel store)

Derive a tenant-wide approval rate from the hash-chained `audit_record` rows the
system already writes (the honest source, never the pending list):

- New read-side helper, mirroring the probation-hold + seeded read-side union
  pattern (`routes_approvals_probation.rs` / `routes_approvals_json_seeded.rs`):
  count `approval.granted` vs `approval.declined` rows for the request tenant →
  `{ total, approvedFraction }`. Tenant fail-closed (never count another tenant's
  rows).
- Surface it as an **additive** `approval_rate` field on the existing
  `GET /api/v1/approvals` response envelope (prefer extending the response over a
  new route), shape matching the existing `ApprovalRateSummary` TS type
  (`web/components/skycot/approvals/ApprovalsKpiStrip.tsx`).
- Web: `ApprovalsOpsDashboard` consumes the server `approvalRate` when present,
  falling back to the existing client `computeApprovalRate(items)` only when the
  field is absent (back-compat, no behaviour change when the server omits it).

Per-worker `approval_rate` already exists in
`crates/argo-cli/src/handlers/worker_metrics.rs` (`granted / (granted + revoked)`).
**REUSE its counting convention** — do not invent a third definition.

## Verifier (checkable, NO THEATER)

- Rust: seed N `approval.granted` + M `approval.declined` audit rows for a tenant,
  call the read, assert `total == N+M` and `approvedFraction == N/(N+M)`; assert a
  cross-tenant row is NOT counted.
- Web: vitest — when the server provides `approvalRate`, the KPI renders the server
  value and does NOT reset to "—" after the pending list empties.

## May-touch
- `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` (+ a small
  read-side helper module if needed; keep every file < 500 LOC)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx`, `ApprovalsKpiStrip.tsx`,
  `web/lib/api/approvals.ts`
- `docs/build-plan/FEATURE_MATRIX.md` `/approvals` row (in-band)

## Out of scope (do NOT do)
- Suppressing legitimately-held probation actions to force the PENDING counter to 0.
  A brand-new teammate's growing queue during its 24h probation window is DEC-025 #2
  working as designed; #2895 already made the copy honest about it.

**LOC budget:** ~150–250. **Priority:** P2 — trust/polish, not a functional or
go-live blocker.
