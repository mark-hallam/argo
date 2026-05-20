# web-tab-title-remaining-inapp-subpages — per-tab metadata for /workers/* /skills /settings /hire

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-tab-title-remaining-inapp-subpages`.
**Priority**: P3 — wave-5 e2e re-run flagged remaining tab-title leak. Wave-5 fix covered /, /team, /audit, /cohorts; this brief covers the rest.
**Sizing**: ~150-250 LOC (per-route sub-layouts + tests).

## Why this brief exists

From the wave-5 e2e re-run:

> Tab title leak persists on /workers/*, /skills, /settings/*, /hire
> steps, plus a few others. Wave-2 SEO fix missed the in-app shell
> pages; wave-5 added (app) default + sub-layouts for /team /audit
> /cohorts. Remaining in-app pages still fall back to the root
> "SKYCOT — Hire your first AI teammate" marketing title.

Per-tab `<title>` requires a server-component sub-layout (client
pages can't export metadata directly — pattern established in wave-2
for /auth/* and wave-5 for (app)/team etc.).

## Goals

Add per-route metadata via sub-layouts for:
1. `/workers` (root list) → "Workers"
2. `/workers/[workerId]` → "${workerName}" (use generateMetadata if possible)
3. `/workers/[workerId]/overview` → "${workerName} · Overview"
4. `/workers/[workerId]/memory` → "${workerName} · Memory"
5. `/workers/[workerId]/skills` → "${workerName} · Skills"
6. `/workers/[workerId]/audit` → "${workerName} · Audit"
7. `/workers/[workerId]/policies` → "${workerName} · Policies"
8. `/workers/[workerId]/escalation` → "${workerName} · Escalation" (once that brief lands)
9. `/skills` → "Skills"
10. `/skills/[skillId]` → "${skillName}" via generateMetadata
11. `/settings` → "Settings"
12. `/settings/profile`, `/members`, `/security`, `/billing`, etc. → per-tab titles
13. `/hire`, `/hire/step/[step]`, `/hire/done/[workerId]` → "Hire", "Hire · step N", "Welcome to ${workerName}"

## Steps

1. For each route with a client-component page.tsx, add a sibling layout.tsx that exports metadata.
2. For dynamic routes (workerId, skillId), use Next.js `generateMetadata({ params })` to read the entity name.
3. Verify in dev: visit each page → check `<title>` in head.

## may-touch

- New layout.tsx files under (app)/workers/, (app)/skills/, (app)/settings/, (hire)/

## must-NOT-touch

- Existing (app)/layout.tsx, /team/layout.tsx, /audit/layout.tsx, /cohorts/layout.tsx (already done wave-5)
- /auth/*/layout.tsx (already done wave-2)

## Acceptance criteria

- [ ] Every in-app subpage has its own per-route `<title>`.
- [ ] Dynamic routes (workers/[id], skills/[id]) use generateMetadata to read entity name.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Manual sanity: visit 6 different in-app subpages → 6 different tab titles.

## References

- Audit: `docs/skycot/audit/e2e-rerun/2026-05-20-wave-5-e2e-rerun.md`
- Pattern: `web/app/(app)/team/layout.tsx` (wave-5 — minimal metadata exporter)
