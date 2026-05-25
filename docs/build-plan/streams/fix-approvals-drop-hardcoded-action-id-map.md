# fix-approvals-drop-hardcoded-action-id-map

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-drop-hardcoded-action-id-map`.
**Source**: e2e-ux-audit (scene-6-claim-6-trust-and-approvals) finding F8 — `docs/skycot/ux-audits/2026-05-26-0820/scene-6-claim-6-trust-and-approvals.md`.

🟡 **P1 — Approvals row click target-IDs come from a hardcoded 8-row map, not the approval payload. The moment a real approval arrives, the click-through breaks unless the server includes `action_id` AND the map is removed.**

## Root cause

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:25-34` defines `SAMPLE_APPROVAL_ACTION_IDS` — a hardcoded `Record<approvalId, actionId>` mapping the 8 seed approvals (`ap_01H9Z0` through `ap_01H9Z7`) to their underlying audit action IDs.

The fallback at line 296:
```ts
const actionId = SAMPLE_APPROVAL_ACTION_IDS[approval.id] ?? approval.id;
```

This means non-sample approvals use the approval's own ID as the action ID — which won't match any audit row, so `/audit/action/<id>` lands on `AuditActionNotFound` for any real-server approval.

For the POC demo this is fine IF the only approvals shown are the 8 seeded ones. But:
1. The contract is half-wired: real payloads MUST carry `action_id`, and the hardcoded fallback masks any time the server omits it (silent breakage).
2. The hardcoded map is load-bearing for the failure mode — leaving it in the codebase means the next refactor reintroduces the bug.
3. `rowToItem` at `web/lib/api/approvals.ts:153-177` DOES support `row.action_id` already — the server contract supports it; the SPA just isn't relying on it.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F8 [P1] — Approvals row click target IDs come from a hardcoded sample map instead of the approval payload's `action_id`

## Fix recipe

1. **Drop the hardcoded map** at `ApprovalsOpsDashboard.tsx:25-34`. Delete `SAMPLE_APPROVAL_ACTION_IDS` entirely.

2. **Rely on `item.actionId` directly** at line 296:
   ```ts
   const actionId = item.actionId ?? null;
   if (!actionId) {
     // No action to drill into — render the row without a click target
     return <ApprovalRowReadOnly approval={item} />;
   }
   ```

3. **Update `ApprovalRow`** to render as a non-button (or with `disabled` styling) when `actionId` is null. The visual affordance should match reality — a row with no drill-in shouldn't look clickable. Use a subdued cursor and remove the hover ring.

4. **Backfill `action_id` on the seed approvals**. Touch `web/lib/sample-approvals.ts` (or wherever the 8 seed approvals are defined). Each of `ap_01H9Z0` through `ap_01H9Z7` should carry its `actionId` field directly — same string the hardcoded map was emitting. This preserves the existing demo path while removing the indirection.

5. **Server contract pin**: confirm `rowToItem` at `web/lib/api/approvals.ts:153-177` reads `row.action_id` (or the canonical field name on the gateway envelope). If the gateway emits a different name (e.g. `target_action_id` or `audit_action_id`), update the field mapping. Pin the field name in a surface contract test.

6. **Log missing action_id in dev** so any server-side omission is visible:
   ```ts
   if (!item.actionId && process.env.NODE_ENV !== "production") {
     console.warn(`[approvals] item ${item.id} has no action_id — drill-in disabled`);
   }
   ```

7. **Test**:
   - Component: `ApprovalRow.test.tsx` — render with `actionId` set → clickable + navigates; render without → non-clickable read-only row.
   - Sample-data smoke: render all 8 seed approvals → all 8 have `actionId` populated; all 8 are clickable.

## Acceptance criteria

- [ ] `SAMPLE_APPROVAL_ACTION_IDS` constant removed from `ApprovalsOpsDashboard.tsx`.
- [ ] All 8 seed approvals in `web/lib/sample-approvals.ts` (or equivalent) carry an explicit `actionId` field.
- [ ] Clicking any of the 8 seed approval rows navigates to `/audit/action/<actionId>` (no regression on Scene 6 demo path).
- [ ] Real-server approvals (when seeded) drill in based on the gateway-provided `action_id` field — no hardcoded map involved.
- [ ] Approvals without an `action_id` render as non-clickable read-only rows (no hover ring, no cursor pointer) — UX matches reality.
- [ ] Dev console warns when an approval lacks `action_id`.
- [ ] Component test passes for both clickable and read-only variants.

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (drop map, update row render)
- `web/components/skycot/approvals/ApprovalRow.tsx` (variant for missing actionId)
- `web/lib/sample-approvals.ts` (backfill actionId on seed)
- `web/lib/api/approvals.ts` (verify rowToItem field mapping)
- `web/components/skycot/approvals/ApprovalRow.test.tsx` (NEW or extended)
- `web/tests/surface_contract_approvals.ts` (field-name pin if not present)

## Out of scope

- Building a dedicated `/approvals/<id>` detail page (separate brief: `fix-approvals-rows-unclickable-no-detail-route` covers this; this brief assumes Option A from that brief — drill into audit-action)
- Gateway-side approval envelope schema bump (assumed `action_id` already exists on the envelope per `rowToItem` reading it)
- Hardcoded KPI strip values (separate brief: `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts` already exists)

## Effort

~40-80 LOC across component + sample data + test. ~1-2 hours.
