# fix-worker-detail-loading-forever-no-error-boundary

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-loading-forever-no-error-boundary`.
**Source**: e2e-ux-audit (onboarding-hire-an-ai-teammate) finding F3 — `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md`.

🔴 **P0 DEMO-BLOCKER — Wizard's redirect target shows nothing but a spinner forever. Operator hits "Hire them" and stares at "Loading" with no end state and no retry affordance.**

## Root cause

The wizard's `first-hire` page at `web/app/(setup)/onboarding/first-hire/page.tsx:51` redirects to `/workers/${res.workerId}` after a successful hire. That page mounts `WorkerOverviewContent` (`/(app)/workers/[workerId]/overview/page.tsx:26`), which:

1. Calls `useWorker(workerId)` → hits `/api/v1/workers/<id>` → **404** (until `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch` lands)
2. Falls back to `findSampleWorker` at `web/lib/api/workers.ts:140`, which is keyed on hardcoded UUIDs of 6 fictional personas (`018f67d0-0000-7000-8000-00000000000{1..6}`)
3. A real UUID like `019e613c-abb5-7733-a945-6d8ae9584ffc` doesn't match → throws `WorkerNotFoundError`
4. There is **no `error.tsx`** for the worker-detail route, so the error bubbles into nothing visible
5. Headless curl of the destination shows the page renders the sidebar + a single `data-testid="workers-loading"` div with no error boundary, no "Worker not found" headline, no link back to `/team`

Result: operator clicks "Hire them" → URL changes to `/workers/<new-id>/overview` → page hangs on Loading indefinitely.

This bug has TWO orthogonal causes:
- (a) the wrong-tenant 404 (covered by `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`)
- (b) the missing error boundary (this brief)

Even after (a) lands, any future 404 or transient failure will reproduce the same forever-spinner. The error boundary is a hardening fix — it MUST exist for the worker-detail route even when the upstream tenant fix is in place.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F3 [P0] — `/workers/{newhire-id}` sits on "Loading" forever; no error boundary, no retry, no link back

## Fix recipe

1. **Add `web/app/(app)/workers/[workerId]/error.tsx`** as a Next.js App Router error boundary. The component receives `{ error, reset }` props. Render:
   - Hero: "We couldn't load this teammate"
   - Sub-line: error.message (if it's a `WorkerNotFoundError`, "This teammate may have been removed or the ID is invalid.")
   - Primary CTA: "Back to your team" → `<Link href="/team">`
   - Secondary CTA: "Try again" → `<button onClick={reset}>`
   - Use the C2-personality-page layout shell so the sidebar + breadcrumb persist

2. **Distinguish `WorkerNotFoundError` from network errors** in `web/lib/api/workers.ts`. Today both look the same; the error boundary should render different copy:
   - `WorkerNotFoundError` → "Not found" hero
   - Network/timeout → "Couldn't reach the gateway" + Retry-with-backoff hint

3. **Add a 30s timeout on `useWorker`** at `web/app/(app)/workers/[workerId]/overview/page.tsx`. If the loader is still pending after 30s, throw a `WorkerLoadTimeoutError` that surfaces via the same error boundary. Prevents the forever-spinner even on network silence.

4. **Add a Playwright test** at `web/tests/e2e/worker-detail-not-found.spec.ts`:
   - Visit `/workers/00000000-0000-0000-0000-000000000000/overview` with no auth
   - Assert page renders the "Couldn't load" hero within 3 seconds
   - Assert "Back to your team" link is present and navigates to `/team` on click

5. **Defensive fallback**: if `useWorker` is still in `isLoading` state after 10 seconds without an error, render a smaller inline "Still loading… something might be wrong" banner above the spinner with a "Reload" button. Belt-and-braces UX.

## Acceptance criteria

- [ ] `web/app/(app)/workers/[workerId]/error.tsx` exists and exports a default React component matching the Next.js error-boundary signature.
- [ ] Visiting any `/workers/<random-uuid>/overview` renders the error hero within 3 seconds (curl + DOM inspection).
- [ ] "Back to your team" link renders and is a real `<Link href="/team">`.
- [ ] "Try again" button calls the `reset()` prop.
- [ ] Network errors and not-found errors render distinct copy.
- [ ] 30s timeout on `useWorker` surfaces via the same error boundary.
- [ ] New Playwright spec passes.
- [ ] Regression test: a freshly-hired worker that exists at `/api/v1/workers/<id>` (after `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch` lands) still renders the normal C2 personality page — error boundary doesn't false-positive.

## May-touch

- `web/app/(app)/workers/[workerId]/error.tsx` (NEW)
- `web/app/(app)/workers/[workerId]/overview/page.tsx` (timeout wiring)
- `web/lib/api/workers.ts` (error subclasses if not already separated)
- `web/tests/e2e/worker-detail-not-found.spec.ts` (NEW)

## Out of scope

- Tenant-resolution fix for the upstream 404 (separate brief: `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`)
- Generic app-shell error boundary (covered by existing `fix-spa-error-pages-and-audit-redactor`)
- Skeleton/loading state polish (a different brief)

## Effort

~80-150 LOC + 1 spec test. ~2 hours. Ship in parallel with the tenant-resolution fix.
