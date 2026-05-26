# fix-demo-banner-copy-branch-on-data-source-or-suppress

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-banner-copy-branch-on-data-source-or-suppress`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/scene-4-and-5-claim-4-and-5.md` F13 [P3] + `docs/skycot/ux-audits/2026-05-27-0015/scene-6-claim-6-trust-and-approvals.md` F8 [P1]).

🟡 **P1 — The "Demo mode — showing sample data. Sign in to see your own workspace." banner shows on every (app)-group page in dev/demo mode, but the claim "showing sample data" is FALSE on pages that are actually showing live gateway data (empty rows + real http.request audit rows). Inconsistency the camera catches: a viewer pausing on `/audit` sees the banner say "sample" while the rows look like real http traffic. Also: the banner is visible in scenes where the operator wants a clean shot.**

## Root cause

`web/components/skycot/DemoBanner.tsx` renders unconditional copy:

> "Demo mode — showing sample data. Sign in to see your own workspace."

The banner is gated on `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` (correct), but the COPY is unconditional. The data behind the page is one of three sources at any moment:

1. Live gateway data (auth-bypass off, gateway up, real envelope) — "showing sample data" is FALSE.
2. Fallback mock-offline data (gateway 404, SPA falls through to seed fixtures) — "showing sample data" is TRUE-ish (it's seed, not customer's data).
3. Mock-bypass data (auth-bypass on, gateway untouched, SPA serves SAMPLE_TEAM/SAMPLE_WORKFLOWS directly) — "showing sample data" is TRUE.

Today the banner says (1) when it's actually (1)/(2)/(3). On Scene 6 the audit page is showing real (empty) gateway data and the banner says "sample" — implicit lie.

Secondary issue (scene-4-5 F13): there's no env-var to suppress the banner for the recording profile. The operator has to dismiss it manually for every take.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/scene-6-claim-6-trust-and-approvals.md` F8 [P1] — Demo banner claim "showing sample data" is false on real-data path
- `docs/skycot/ux-audits/2026-05-27-0015/scene-4-and-5-claim-4-and-5.md` F13 [P3] — Demo banner persists on every (app) page in dev

## Fix recipe

Two layered changes; ship both.

**Layer 1 — branch the copy on data source**:

1. Add a `dataSource` prop to `DemoBanner.tsx` or have the banner read a context value that pages set via a `DataSourceProvider`.
2. Copy variants:
   - `dataSource === "live"` (gateway returns non-empty real data): "Demo mode — sign in to see your own workspace." (no "sample data" claim)
   - `dataSource === "mock-offline"` (gateway 404 → seed fallback): "Demo mode — gateway is empty; showing seeded sample data." OR "Demo mode — backend unreachable, showing seed data."
   - `dataSource === "mock-bypass"` (auth-bypass active → SAMPLE_*): "Demo mode — showing curated sample data. Sign in to see your own workspace." (today's copy)
3. The page-level data hook (e.g. `useTeam`, `useApprovals`, `useAudit`) already knows the source (each `fetch*` helper returns `{items, source}`). Thread `source` to the banner via context or a shared atom.

**Layer 2 — add a suppress env var for recording profiles**:

1. Add `NEXT_PUBLIC_HIDE_DEMO_BANNER=1`. When set, `DemoBanner` returns null regardless of other state.
2. Document in `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-flight: "Set `NEXT_PUBLIC_HIDE_DEMO_BANNER=1` in `.claude/launch.json` before recording, OR press × on the banner once."
3. Optionally make the dismiss state persist in `sessionStorage` so the banner stays dismissed across navigations within a single recording take (today it re-renders on every page load).

## Acceptance criteria

- [ ] On `/audit` against the live (empty) gateway, the banner does NOT claim "sample data" — uses the live-data variant.
- [ ] On `/team` with auth-bypass active (mock-bypass), banner reads "showing curated sample data".
- [ ] On `/team` with gateway 404 (mock-offline), banner reads "backend unreachable, showing seed data" (or equivalent).
- [ ] Setting `NEXT_PUBLIC_HIDE_DEMO_BANNER=1` and reloading: banner does not render.
- [ ] Dismissing the banner persists for the session (no re-render on navigation).
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec vitest run components/skycot/DemoBanner` covers all three copy branches + the env-var hide.

## May-touch

- `web/components/skycot/DemoBanner.tsx` (~40 LOC for variants + env-var suppress)
- `web/lib/skycot/data-source-context.tsx` (new file ~30 LOC for context)
- `web/lib/api/*` (~10 LOC each to expose source via context — or fold into existing `{items, source}` envelope)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (~3 LOC checklist)
- `web/components/skycot/__tests__/DemoBanner.test.tsx` (~50 LOC)

## Out of scope

- The sidebar-overlap layout bug (covered by `fix-demo-banner-sidebar-overlap-desktop`)
- The TeamDataSourceChip on /team (covered by `fix-team-page-mock-offline-chip-suppress-during-recording`)
- The env-var name drift causing mock-bypass to not fire (covered by `fix-claude-launch-json-env-var-naming-mismatch` + `fix-web-mock-bypass-env-var-rename`)

## Effort

S. ~150 LOC + ~50 LOC test. ~2 hours.
