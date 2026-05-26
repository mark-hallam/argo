# fix-audit-action-not-found-page-fallback-routing

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-action-not-found-page-fallback-routing`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-6-claim-6-trust-and-approvals.md` F10 [P2].

🟢 **P2 — `web/components/skycot/audit/AuditActionNotFound.tsx` ships a clean in-shell 404 surface, but in the current demo + auth-bypass config it is unreachable. Any unknown action id falls through to `placeholderAuditDetail()` which fabricates a fake provenance card (see `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`) instead of routing to the not-found page.**

## Root cause

`web/lib/api/audit-detail.ts:99-101` short-circuits to `mockDetail()` in demo+bypass; for any action id NOT in the canonical `SAMPLE_AUDIT_DETAILS` table, the mock-path falls through to `placeholderAuditDetail()` which synthesises a fake provenance card. The `<AuditActionNotFound>` component is correctly built and tested but never rendered.

Two issues:

1. The fall-through to `placeholderAuditDetail()` IS the bug from `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance` (different brief covers the fabrication concern). This brief covers the ROUTING side: any unknown id should land on `<AuditActionNotFound>`, not on a synthesised card.
2. Once `fix-spa-demo-live-mode-route-real-gateway-on-camera` lands, the real gateway will 404 on unknown ids and the not-found surface SHOULD render naturally — but worth a regression test that it still does, since the not-found page hasn't been exercised end-to-end in demo+bypass.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F10 [P2] — `AuditActionNotFound` is only reachable via gateway 404 (so today: never)

## Fix recipe

1. **In `audit-detail.ts:mockDetail()` (or equivalent)**, when the requested `actionId` does NOT appear in `SAMPLE_AUDIT_DETAILS`, return a not-found envelope `{ kind: "not_found", actionId }` INSTEAD of falling through to `placeholderAuditDetail()`.
2. **In `web/app/(app)/audit/action/[actionId]/page.tsx` (verify path)**, route the `not_found` envelope to the `<AuditActionNotFound>` component.
3. **Regression test** at `web/playwright/audit/action-not-found.spec.ts`: navigate to `/audit/action/act_bogusid_does_not_exist` and assert the `<AuditActionNotFound>` shell renders with the action id echoed back.
4. **Once `fix-spa-demo-live-mode-route-real-gateway-on-camera` lands**, add a second Playwright check under `DEMO_LIVE=1` mode that a real gateway 404 also lands on `<AuditActionNotFound>`.

## Acceptance criteria

- [ ] Navigating to `/audit/action/act_bogus_id` lands on `<AuditActionNotFound>`, NOT on a fabricated provenance card.
- [ ] The not-found shell echoes the action id back.
- [ ] Playwright spec passes in both `DEMO_LIVE=0` and (once that brief lands) `DEMO_LIVE=1`.
- [ ] No regression on the canonical-seeded action ids (they still render real detail).

## May-touch

- `web/lib/api/audit-detail.ts` (mockDetail not-found return path)
- `web/app/(app)/audit/action/[actionId]/page.tsx` (route not-found envelope)
- `web/playwright/audit/action-not-found.spec.ts` (NEW)
- `web/components/skycot/audit/AuditActionNotFound.tsx` (verify props; no change expected)

## Out of scope

- Removing the placeholder fabrication itself (covered by `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`)
- Demo-live mode wiring (covered by `fix-spa-demo-live-mode-route-real-gateway-on-camera`)

## Effort

~30-60 LOC + 1 Playwright spec. ~1-2 hours. P2 because unreachable today but a real correctness regression once paths converge.
