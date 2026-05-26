# fix-audit-worker-filter-uses-uuid-not-slug

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-worker-filter-uses-uuid-not-slug`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-1-and-2-claim-1-and-2.md` F11 [P1].

🟠 **P1 — The "Open in Audit timeline →" link at the bottom of the worker Activity tab uses the raw UUID: `/audit?worker=018f67d0-0000-7000-8000-000000000001&range=7d`. If the operator copies this URL into Slack or screen-shares the address bar, the camera reads a 36-character UUID. Internal-IA leak on a Scene 1+2 surface.**

## Root cause

`web/app/(app)/workers/[workerId]/activity/page.tsx:69` constructs the link as `/audit?worker=${workerId}&range=7d`. `workerId` is the canonical worker UUID (e.g. `018f67d0-0000-7000-8000-000000000001` for Mariah).

The audit page (`/audit`) reads `?worker=` and filters. The audit-feed gateway likely accepts UUIDs only — but a slug-based equivalent (`?worker=mariah`) would be more shareable and less leaky.

Two policy options:
- (a) Add slug support to the audit-page filter — accepts both UUID and slug.
- (b) Keep UUID in the URL but render only the slug in the visible link / breadcrumb.

Recommended: (a) — slugs become shareable URLs that don't expose internal IDs.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F11 [P1] — `/audit?worker=<id>` link from the Activity tab uses the raw UUID

## Fix recipe

1. **Add a worker-slug lookup table** at `web/lib/sample-personas.ts` (or extend the existing canonical-persona registry). Each persona has a `slug` field (e.g. `mariah`, `peta`, `priya`, `jamal`, `nori`).
2. **Update the Activity tab link** at `web/app/(app)/workers/[workerId]/activity/page.tsx:69`:
   ```tsx
   const slug = lookupWorkerSlug(workerId) ?? workerId;  // fall back to UUID if no slug
   const href = `/audit?worker=${slug}&range=7d`;
   ```
3. **Extend the audit-page filter** at `web/app/(app)/audit/page.tsx`:
   ```tsx
   const workerParam = searchParams.worker;
   const workerId = isUuid(workerParam)
     ? workerParam
     : lookupWorkerIdFromSlug(workerParam) ?? workerParam;
   ```
4. **Verify the gateway accepts both** — `/api/v1/audit?worker=<value>` should either accept UUIDs only (with SPA-side slug→UUID translation) or accept both. SPA-side translation is simpler and the audit-page already has access to the persona registry.
5. **Add a Playwright spec** `web/playwright/audit/worker-filter-slug-and-uuid.spec.ts`:
   - Visit `/audit?worker=mariah&range=7d`; assert rows filtered to Mariah's actions.
   - Visit `/audit?worker=018f67d0-0000-7000-8000-000000000001&range=7d`; assert same filtered set.
   - On a worker activity tab, click "Open in Audit timeline" and assert the URL uses the slug, not the UUID.

## Acceptance criteria

- [ ] Worker Activity tab link uses the slug, not the UUID.
- [ ] `/audit?worker=mariah` filters correctly.
- [ ] `/audit?worker=<UUID>` continues to work (backward compatibility).
- [ ] New Playwright spec passes.
- [ ] No regression on the audit feed rendering with no `worker=` filter.

## May-touch

- `web/lib/sample-personas.ts` (slug field — verify exists or add)
- `web/lib/persona-registry.ts` (lookup helpers `lookupWorkerSlug` / `lookupWorkerIdFromSlug` — NEW or extend existing)
- `web/app/(app)/workers/[workerId]/activity/page.tsx:69`
- `web/app/(app)/audit/page.tsx` (filter parsing)
- `web/playwright/audit/worker-filter-slug-and-uuid.spec.ts` (NEW)

## Out of scope

- Renaming `/workers/[workerId]/...` route segments to use slugs (large refactor, defer).
- Audit feed envelope shape (covered by `fix-web-audit-feed-envelope`).
- Audit feed action-kind variety (covered by `fix-audit-feed-action-kind-variety`).

## Effort

~60-120 LOC + 1 spec. ~2-4 hours. P1 — UUID leak in operator-visible URLs is a credibility crack on a screen-share.
