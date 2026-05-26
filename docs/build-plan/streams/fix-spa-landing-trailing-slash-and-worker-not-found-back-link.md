# fix-spa-landing-trailing-slash-and-worker-not-found-back-link

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-landing-trailing-slash-and-worker-not-found-back-link`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/onboarding-hire-an-ai-teammate.md` F11 [P2] + F14 [P2].

🟡 **P2 — Two low-leverage polish issues on the marketing/workers routes: (a) `/landing` issues an unnecessary 308 redirect on the trailing-slash variant — cold visits to a shared link with `/landing/` add a redirect hop and ~50ms TTFB. (b) The WorkerNotFound and worker-detail error.tsx pages link "Back to team" → `/team`, but the sidebar nav entrypoint is "Workers" → `/workers`. After a not-found, the operator is sent to a different mental-model surface than the one they clicked from.**

## Root cause

### F11 — trailing slash

`curl http://127.0.0.1:3002/landing` returns 200 (no slash). `curl http://127.0.0.1:3002/landing/` returns 308 → `/landing`. Cold visits to a shared link with the trailing slash form take an extra redirect hop. The canonical link form used in internal nav is no-slash, so external links matching internal-nav style work — but any human typing `/landing/` or any social-media URL preview that appends `/` pays the redirect cost.

Two fixes:
- (a) Set `trailingSlash: false` in `web/next.config.ts` so all paths are canonicalized no-slash and 308s collapse to 404s for non-canonical forms.
- (b) Add `trailingSlash: false` AND a `redirects()` rule that 301s common trailing-slash forms.

Recommended: just set `trailingSlash: false` if it isn't already; Next.js defaults usually canonicalize.

### F14 — worker-not-found back-link

`web/app/(app)/workers/[workerId]/error.tsx:36-41` renders a "Back to team" link pointing at `/team`. Same in `web/components/skycot/WorkerNotFound.tsx:34-37`. The sidebar nav says "Workers" → `/workers`. The operator clicked from "Workers", a 404 surface sends them to `/team`. Confusion.

Note: `/workers` redirects to `/team` server-side when populated (covered by `fix-workers-redirect-to-team-url-flicker`), so functionally identical — but the back-link should mirror the click-from surface for mental model consistency.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F11 [P2] — `/landing` route requires trailing-slash redirect
- onboarding-hire-an-ai-teammate F14 [P2] — Worker-detail error fallback links to `/team`, not `/workers`

## Fix recipe

### F11 trailing-slash

1. **Inspect `web/next.config.ts`** for `trailingSlash` setting.
2. **Set `trailingSlash: false` explicitly** (likely already the implicit default but worth being explicit):
   ```ts
   const nextConfig = {
     trailingSlash: false,
     // ... other settings
   };
   ```
3. **Verify** with `curl -sI http://127.0.0.1:3002/landing` — should return 200 directly.
4. **Verify** `curl -sI http://127.0.0.1:3002/landing/` — should return 404 OR a 308 that's idempotent. If Next.js is double-redirecting (308 → 308), fix the redirect rule.

### F14 back-link

5. **Change the back-link target** at `web/app/(app)/workers/[workerId]/error.tsx:36-41`:
   ```tsx
   <Link href="/workers">Back to workers</Link>
   ```
6. **Same change** at `web/components/skycot/WorkerNotFound.tsx:34-37`.
7. **Note**: `/workers` redirects to `/team` server-side, so the user lands on `/team` either way — but the back-link's _intent_ matches the nav-bar click that brought them here.

### Tests

8. **Add a Playwright spec** at `web/playwright/worker-detail/not-found-back-link.spec.ts`:
   - Visit `/workers/00000000-0000-0000-0000-000000000000/overview` (a never-existent id).
   - Assert the page contains "Back to workers" link with `href="/workers"`.
9. **Add a Playwright spec** at `web/playwright/marketing/trailing-slash-canonical.spec.ts`:
   - `curl /landing` → 200, single hop.
   - `curl /landing/` → either 404 or single-hop 308.

## Acceptance criteria

- [ ] `/landing` returns 200 directly (no redirect hop).
- [ ] `/landing/` returns either 404 OR a single 308 (no double-redirect).
- [ ] WorkerNotFound + error.tsx back-link points at `/workers`, not `/team`.
- [ ] Both new specs pass.

## May-touch

- `web/next.config.ts` (trailingSlash)
- `web/app/(app)/workers/[workerId]/error.tsx`
- `web/components/skycot/WorkerNotFound.tsx`
- `web/playwright/worker-detail/not-found-back-link.spec.ts` (NEW)
- `web/playwright/marketing/trailing-slash-canonical.spec.ts` (NEW)

## Out of scope

- `/workers` → `/team` server-side redirect (covered by `fix-workers-redirect-to-team-url-flicker`).
- Workers / Team / Cohorts IA consolidation (deferred; covered by F22 whole-site as P3).
- Sidebar nav entry "Workers" removal (covered by `fix-workers-team-route-ambiguity-and-breadcrumb-drift`).

## Effort

~20-50 LOC + 2 specs. ~1-2 hours. Both P2 polish.
