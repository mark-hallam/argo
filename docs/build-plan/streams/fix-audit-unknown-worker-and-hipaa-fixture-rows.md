# fix-audit-unknown-worker-and-hipaa-fixture-rows

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-unknown-worker-and-hipaa-fixture-rows`.
**Source**: e2e-ux-audit Agent 1 finding F13 + Agent 2 findings F13 + F17 + F18 + Agent 4 finding F9 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md` + `docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md` + `docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`).

🟡 **P1 — Live audit rows are all "unknown / hipaa_compliance_check". Scene 2 + Scene 6 drill-in vocabulary is wrong.**

## Root cause

Curling `/api/v1/audit?range=7d` against the live gateway returns 7+ identical rows of:

```json
{
  "worker": {"id": "", "name": "unknown"},
  "verb": "hipaa_compliance_check",
  "action_type": "hipaa_compliance_check"
}
```

A healthcare-compliance action on a marketing-intern tenant. Three coupled problems:

1. **Worker reference is empty.** `worker.id === ""` and `worker.name === "unknown"`. The SPA's avatar at `ActivityRow.tsx` renders "UN" (Unknown initials). Per the v1.0 runbook Scene 2 "Verify the audit row's payload is REAL (real worker_id, real tenant_id, real action description — not a fixture)" — every row fails this check.
2. **Verb column renders the raw machine action key.** Each row shows `hipaa_compliance_check` (the action_type string) rather than a human verb like "Ran HIPAA compliance check". Scene 6 narrates the camera reading audit verbs aloud — these should be human-readable.
3. **The hipaa_compliance_check rows are stray test fixtures** that landed in the demo tenant. They're inappropriate for a marketing-intern tenant context regardless of attribution.

Net effect: the demo MP4 cannot show real audit drill-in narrative — every row is `unknown / hipaa_compliance_check`. Either the gateway needs richer seed data (per the operator-locked `feedback_no_theater_in_demo` Option B = real backend seed CLI) OR the demo MUST run with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` to fall back to `SAMPLE_AUDIT_ROWS` (which IS narratively rich).

## Findings this brief closes

- Agent 1 F13 [P2] — `/audit` rows show worker `UN / unknown` because audit-side worker_id is empty string
- Agent 2 F13 [P0] — Audit row attribution is "unknown" worker with no id; rows all show `hipaa_compliance_check` on a marketing-intern tenant
- Agent 2 F17 [P2] — Audit row "verb" column shows the raw machine action key (`hipaa_compliance_check` instead of "Ran HIPAA compliance check")
- Agent 2 F18 [P2] — Audit row avatar shows "UN" (Unknown) instead of worker initial (tied to F13)
- Agent 4 F9 [P1] — Live audit rows have empty `worker.name` ("unknown") and identical `verb` ("hipaa_compliance_check")

## Fix recipe

Three coordinated sub-fixes:

1. **Purge the stray hipaa_compliance_check rows + re-seed honest rows.** Add to the demo-seed CLI (per the `fix-demo-seed-cli-for-real-roster` brief / existing `streams/demo-seed-cli-*` work) a step that:
   - DELETEs all audit rows on the demo tenant
   - INSERTs a representative set of rows tied to the actual roster (Mariah/Peta/Priya/Jamal/Nori once they're seeded), with verbs that match the verticals: `posted_listing`, `screened_candidate`, `approved_refund`, `qualified_lead`, `triaged_ticket`, etc.
   - Each row carries a real `worker_id` (ULID matching the seeded worker) and a real `tenant_id`.

2. **System-action attribution.** For audit rows that legitimately have no worker (e.g. background compliance scans), the gateway should set `worker: {id: "system", name: "System"}` not `{id: "", name: "unknown"}`. SPA-side, the avatar renders "SY" + a "system" badge rather than "UN".

3. **Human-readable verb mapping.** Add a `verbForAction(actionType)` mapping (either gateway-side as a `display_verb` field on the audit response, or SPA-side as a lookup table). Pattern:
   ```
   posted_listing → "Posted listing"
   screened_candidate → "Screened candidate"
   approved_refund → "Approved refund"
   hipaa_compliance_check → "Ran HIPAA compliance check"
   ```
   Prefer the gateway-side `display_verb` so all surfaces (audit list, audit detail, replay header) read the same string.

## Acceptance criteria

- [ ] No `hipaa_compliance_check` rows on the demo tenant after re-seed (unless a healthcare-vertical demo exists).
- [ ] Every audit row on the demo tenant has a real `worker_id` (matching a seeded roster member) OR `worker.id === "system"` (with name "System").
- [ ] No row has `worker: {id: "", name: "unknown"}`.
- [ ] Audit list verb column renders human-readable strings, not raw machine keys.
- [ ] curl-test: `GET /api/v1/audit?range=7d` returns rows with `display_verb` field (or equivalent human-readable string).
- [ ] Avatar in `ActivityRow.tsx` renders correct initials for the real worker, or "SY" for system actions, never "UN".

## May-touch

- `crates/argo-demo-seed/src/audit_rows.rs` (or wherever audit seed lives — likely under `crates/argo-cli/src/cli/demo_seed/`)
- `crates/argo-cli/src/handlers/gateway/routes_audit*.rs` (system-action handling, optional `display_verb` field)
- `web/components/skycot/audit/ActivityRow.tsx` (avatar fallback, verb rendering)
- `web/lib/api/audit.ts` (type update if `display_verb` is added gateway-side)

## Out of scope

- Audit hash-chain prev-this (separate brief: `fix-audit-hash-chain-prev-this-hover-and-live-rows`)
- Audit list/detail endpoint ID mismatch (separate brief: `fix-audit-list-detail-endpoint-id-mismatch`, already authored)
- Replay page fabrication (separate brief: `fix-replay-page-fabricated-forensic-report-for-unknown-action-ids`, already authored)
- Re-seeding the worker roster itself (separate brief: `fix-demo-seed-cli-for-real-roster`)

## Effort

~100-200 LOC across crates + web + seed CLI. ~1-2 hours.
