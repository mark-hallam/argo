# fix-audit-action-detail-demo-mode-fallback

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-action-detail-demo-mode-fallback`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` F2 [P0].

🔴 **P0 DEMO-BLOCKER — Click any row in `/audit` → server-rendered 500 page: "We couldn't load this audit action." Scene 6's hero beat (row → drill-in → see what really happened → hash-chain verify) is a hard 500 every time.**

## Root cause

The audit feed (`/audit`) successfully renders 13 rows in the TODAY section. Clicking the first row navigates to `/audit/action/019e63c2cd537671847bbc84c12cbbd5` and Next.js renders the 500 error page with copy "Something went wrong fetching the action's details. Retrying usually fixes it; otherwise the audit feed is always available."

The detail route page (`web/app/(app)/audit/action/[actionId]/page.tsx`) fetches via `fetchAuditDetail(actionId)` which calls the gateway at `/api/v1/audit/{actionId}`. Two failure modes both produce the 500:

1. **No demo-mode fallback wired.** When the gateway returns 4xx or 5xx, `fetchAuditDetail` re-throws and the server-rendered page becomes the Next.js 500 boundary. There is no `if (isDemoMode()) return mockDetail(actionId)` branch the way the audit-feed and team routes have.
2. **The rendered list-row ID format does not match the gateway's expected schema.** The list endpoint returns rows with IDs like `019e63c2cd537671847bbc84c12cbbd5` (32-char hex) but the live gateway under demo mode either has no audit row with that ID or the route handler expects a different format. Either way the gateway 404 or 500s and the SPA has no recovery path.

Cross-references:
- `web/lib/api/audit-detail.ts:58-77` — `fetchAuditDetail` with no fallback
- `web/app/(app)/audit/action/[actionId]/error.tsx` — the 500 boundary the operator sees
- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs` — the gateway handler that 5xxs

## Findings this brief closes

- chrome-live-walk-and-playwright F2 [P0] — Audit drill-in returns 500

## Fix recipe

1. **Add a demo-mode short-circuit in `fetchAuditDetail`** (`web/lib/api/audit-detail.ts`):
   - If `isDemoMode()` is true, look up the action ID in `SAMPLE_AUDIT_DETAILS` (or build it on the fly from the list-row data the SPA already has in cache)
   - If found, return the demo detail without hitting the gateway
   - If not found, return a synthetic detail object with `summary: { action_type: "audit.detail.viewed", ... }` so the page renders SOMETHING rather than 500-ing

2. **Wrap the gateway call in a try/catch in the page component** so a fetch failure renders a graceful "This audit row's details aren't available — drill into another row" message rather than the Next.js 500 boundary. The wide error boundary at `error.tsx` is the wrong UX for "we couldn't fetch one record".

3. **Align the list-row ID schema with the detail-route schema.** If the list endpoint returns 32-char hex IDs but the detail endpoint expects a different format, the SPA's `formatActionId(row.id)` (or equivalent) needs to normalise. Investigate:
   - `web/lib/api/audit.ts` — what `id` does the list normaliser produce?
   - `web/components/skycot/ActivityRow.tsx` — what href does the click handler use?
   - `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs` — what shape does the handler match?

4. **Add a Playwright spec** at `web/playwright/audit/drill-in-renders.spec.ts`:
   - Run with demo mode on
   - Navigate to `/audit`, click the first row
   - Assert URL advances to `/audit/action/{id}` AND the page renders a `[data-testid="audit-detail-summary"]` element (not the 500 boundary)
   - Assert no console errors

## Acceptance criteria

- [ ] With demo mode on, clicking any row in `/audit` lands on `/audit/action/{id}` and renders the detail page (not a 500)
- [ ] The detail page shows at minimum: action_type, timestamp, worker name, verb, status
- [ ] If the gateway fails, the SPA renders a graceful in-page empty state, NOT the global 500 error boundary
- [ ] New Playwright spec passes

## May-touch

- `web/lib/api/audit-detail.ts` (add demo-mode fallback)
- `web/lib/sample-audit-detail.ts` (extend to cover the demo list-row IDs)
- `web/app/(app)/audit/action/[actionId]/page.tsx` (try/catch + graceful empty state)
- `web/app/(app)/audit/action/[actionId]/error.tsx` (narrow the boundary's scope)
- `web/playwright/audit/drill-in-renders.spec.ts` (NEW)

## Out of scope

- Detail envelope deserialization mismatch (covered by `fix-audit-detail-adapter-deserialize-envelope`)
- Hash-chain hover affordance (covered by `fix-audit-hash-chain-prev-this-hover-and-live-rows`)
- Replay 400 vs network error mapping (covered by `fix-replay-page-fabricated-forensic-report-for-unknown-action-ids`)

## Effort

~150-250 LOC. ~4-6 hours. P0 because Scene 6's climactic camera move is currently a 500.
