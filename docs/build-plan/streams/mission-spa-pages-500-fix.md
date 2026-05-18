# mission-spa-pages-500-fix — fix /skills, /approvals, /settings server crashes (P1)

**Owner**: claude. **Sizing hint**: server-side render investigation + bugfix.
**Depends on**: `mission-gateway-cors-preflight` MERGED (so the SPA can actually fetch gateway data without CORS getting in the way during testing).
**Authored against**: origin/main `91f9d032` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-spa-pages-500-fix`.
**Source**: 2026-05-18 browser UX audit — three authenticated routes hit the branded 500 page instead of rendering: `/skills`, `/approvals`, `/settings`.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — three of the seven sidebar destinations crash. Three of seven is half the product surface.
- **Commercial usability** (#5) — a buyer clicking around the demo will hit one of these within 30 seconds.

## Scope

P1. Diagnose the server-side crash on each of the three routes; fix each. Likely root cause is one of:
- Server component calling a missing API endpoint
- Missing environment variable used at render time
- Unhandled exception when the gateway is unreachable / returns 404

Add a fallback "empty state" UI for each page so a fresh workspace (no skills / no approvals / no settings yet) renders something useful instead of 500.

## What lands

1. **Reproduce each 500** locally with a fresh workspace + running gateway, capture the exception stack trace from `.next/server` logs.
2. **Fix each crash** — likely defensive: catch the failed fetch, render empty state.
3. **Add empty-state component** for each page (e.g. "No skills yet — install your first from the marketplace" / "No approvals waiting"). Use the brand language from #1813's 500 page work.
4. **Add a vitest test per page** that mounts with an empty data response and confirms no exception.
5. **Tier-1 visual check**: navigate to each page in a fresh-workspace browser, confirm the empty state renders.

LOC: ~250.

## Files you may touch

- `web/app/(app)/skills/page.tsx` (or wherever the route lives). ≤80 modified LOC.
- `web/app/(app)/approvals/page.tsx`. ≤80 modified LOC.
- `web/app/(app)/settings/page.tsx`. ≤80 modified LOC.
- `web/components/skycot/EmptyState.tsx` (NEW or extend an existing). ≤80 LOC.
- `web/__tests__/skills/page.test.tsx`, `approvals/page.test.tsx`, `settings/page.test.tsx` (NEW or extend). ≤200 LOC total.

## Files you must NOT touch

- The 500 page itself (`web/app/error.tsx` or similar) — already excellent per #1813.
- Other routes outside the three.
- The gateway side — empty-data responses come from a healthy gateway returning an empty list, not a crash.

## Acceptance criteria

- [ ] `/skills` renders empty state on fresh workspace, no 500.
- [ ] `/approvals` renders empty state on fresh workspace, no 500.
- [ ] `/settings` renders default settings page, no 500.
- [ ] vitest tests cover each empty-state path.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-spa-pages-500-fix`
- Title: `fix(web): empty-state handling for /skills /approvals /settings (mission-spa-pages-500-fix)`
- Worker verification: pnpm typecheck + tests + manual browser walk against a fresh workspace.

## Hidden constraints

- **Empty state is itself a brand moment.** Don't show a sad face. Show next-step actions ("Install your first skill from the marketplace →") matching the marketing voice.
- **Don't paper over real bugs.** If the 500 is from a missing endpoint, file a follow-up brief for the gateway-side fix. This brief is "make the page survive" — the gateway gap is separate.
