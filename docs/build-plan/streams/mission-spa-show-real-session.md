# mission-spa-show-real-session — replace seeded mock data with real session data (P1)

**Owner**: claude. **Sizing hint**: cross-component session-binding refactor.
**Depends on**: `mission-gateway-cors-preflight` MERGED.
**Authored against**: origin/main `91f9d032` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-spa-show-real-session`.
**Source**: 2026-05-18 browser UX audit — every authenticated SPA page (Audit, Workers/Team, Cohorts, sidebar bottom-left) shows seeded mock data ("Sam Harlow · Acmeco · Owner", 6 fake workers, 15 fake audit rows) regardless of who is signed in. A fresh signup sees somebody else's workspace.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — first-impression correctness. A buyer doing a free trial sees "Sam Harlow" in the bottom-left and wonders what they're looking at.
- **Commercial usability** (#5) — the mock data is great for the recorded demo, but it has to be CONDITIONAL: real-data path for real sessions, seeded-data path for demo recording only.

## Scope

P1. Two-phase fix:

### Phase 1 — Identify mock-data sources

Grep for hardcoded names + IDs (`Sam Harlow`, `Acmeco`, `Mariah`, `Priya`, `Jamal`, `SUP-29382`, `INV-2024-08741`, `JD-2024-08`). Catalog every component still serving mock data.

### Phase 2 — Wire each to real session data

For each component:
- Read the actual session identity from NextAuth + the actual workspace from the gateway
- Wire the data fetch from the appropriate `/api/v1/*` endpoint (or `/workers`, `/audit/recent`, etc.)
- Add an empty state for fresh workspaces (folds into `mission-spa-pages-500-fix`'s empty-state pattern)

### Phase 3 — Optional: demo mode toggle

Add `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` env var that switches every page back to the seeded mock data. Used for the recorded v1 demo, off in production.

If demo mode is on: page header reads "Demo mode — sample data" in a small dismissible banner.

LOC: ~600 across multiple components.

## Files you may touch

- `web/app/(app)/page.tsx` and dashboard widgets — replace mock data fetches with real gateway calls. ≤200 modified LOC.
- `web/app/(app)/audit/*` — wire to `/api/v1/audit/feed` (or `/audit/recent`). ≤80 modified LOC.
- `web/app/(app)/team/*` (or `/workers/*`) — wire to `/api/v1/workers` list. ≤80 modified LOC.
- `web/app/(app)/cohorts/*` — wire to `/api/v1/cohorts` (or its real path). ≤80 modified LOC.
- `web/components/skycot/UserPill.tsx` — read identity from NextAuth session. ≤40 modified LOC.
- `web/lib/skycot/use-session.ts` (NEW or extend) — single hook for session+workspace. ≤80 LOC.
- `web/components/skycot/DemoBanner.tsx` (NEW) — demo-mode banner. ≤40 LOC.

## Files you must NOT touch

- The 5+ vertical templates' seed data — that's product data, not UI mock.
- The mock fixtures used by tests — those stay.

## Acceptance criteria

- [ ] Logged-in user's email + name shown bottom-left (not Sam Harlow).
- [ ] `/audit` shows real chain rows for the current tenant (empty state on fresh workspace).
- [ ] `/team` shows real workers (empty state with "Hire your first worker" CTA on fresh).
- [ ] `/cohorts` shows real cohorts (empty state on fresh).
- [ ] `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` re-enables seeded mock data + shows demo banner.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-spa-show-real-session`
- Title: `feat(web): wire authenticated pages to real session data + demo-mode toggle (mission-spa-show-real-session)`
- Worker verification: pnpm checks + manual browser walk: fresh workspace shows empty states; existing workspace shows real data; `SKYCOT_DEMO_MODE=1` restores mock.

## Hidden constraints

- **Demo mode preserved for v1.0 recording.** Operator likely already recorded against the mock data — must not break replays.
- **Empty states are brand moments** — see `mission-spa-pages-500-fix` for the empty-state voice.
- **The mock data lives in the page components today, not in the gateway.** This brief moves the source of truth to the gateway.
