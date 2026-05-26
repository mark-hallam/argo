# fix-workers-redirect-to-team-url-flicker

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workers-redirect-to-team-url-flicker`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-1-and-2-claim-1-and-2.md` F9 [P1].

🟡 **P2 — `/workers` client-side redirects to `/team` when workers exist. Runbook says "open /workers"; viewer sees a brief spinner then the URL flips to `/team`. Small but camera-visible URL change that breaks the narration.**

## Root cause

`web/app/(app)/workers/page.tsx:25-27` calls `router.replace("/team")` when workers exist. The redirect runs after hydration → URL bar flashes `/workers` → `/team`. Either canonicalize on one route or rewrite the runbook.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F9 [P1] (downgraded to P2 — camera-visible but not demo-blocking) — `/workers` redirects to `/team`

## Fix recipe

1. **Operator decision**: should the canonical route be `/workers` or `/team`?
2. **If `/workers`**: invert the redirect — `/team` redirects to `/workers`. Update `nav-items.tsx` to use `/workers`. Update sidebar label to match.
3. **If `/team`**: rewrite the runbook narration "open `/team`" instead of `/workers`. Add a `next.config.js` rewrite so `/workers` server-side redirects to `/team` (no client-side URL flicker).
4. **Test**: e2e smoke asserts no URL change happens after initial load.

## Acceptance criteria

- [ ] Single canonical route; no client-side URL flip.
- [ ] Sidebar nav points to canonical route.
- [ ] Runbook updated to match.
- [ ] No regression on the rich worker deck rendering.

## May-touch

- `web/app/(app)/workers/page.tsx` (remove client redirect)
- `web/app/(app)/team/page.tsx` (verify)
- `next.config.js` (server-side rewrite if needed)
- `web/components/skycot/nav-items.tsx` (label/href)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (narration tweak)

## Out of scope

- Worker deck rendering (separate)
- Team-page hero KPIs (`fix-team-page-hero-hardcoded-weekly-actions-hours-saved`)

## Effort

~20-40 LOC + 1 e2e. ~30min once operator picks the route.
