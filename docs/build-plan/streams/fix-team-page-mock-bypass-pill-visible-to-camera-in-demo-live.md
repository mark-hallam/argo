# fix-team-page-mock-bypass-pill-visible-to-camera-in-demo-live

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-team-page-mock-bypass-pill-visible-to-camera-in-demo-live`
**Source:** UX e2e audit 2026-05-28-0252 (scene-1-and-2 F10)

🟡 **P2 — `/team` page header reads "Team `MOCK-BYPASS` · 5 teammates working" with a literal pill exposing the `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` flag to the camera. It's a dev-mode telltale that doesn't belong in the demo recording — the demo banner above already says "Demo mode — showing sample data".**

## Root cause

The `/team` page header (likely `web/app/(app)/team/page.tsx` or its header child component) renders a `<DataSourceChip>` keyed off `useDataSource()` returning `mock-bypass`. The chip is intended for dev visibility but currently fires whenever auth-bypass is on — including the recording-day preview environment that runs with `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` (the recording-mode flag added by `fix-spa-demo-live-mode-route-real-gateway-on-camera` MERGED #2432).

For the v1.0 demo MP4 the recording profile sets BOTH `auth-bypass=1` AND `demo-live=1`. The demo banner above the page communicates "Demo mode" to the audience; the second `MOCK-BYPASS` pill on the header is redundant and reads as a dev artefact.

## Findings this brief closes

- scene-1-and-2 F10 [P2] — `MOCK-BYPASS` pill on /team header visible to camera during demo recording

## Fix recipe

1. **Hide the `MOCK-BYPASS` pill when `isDemoLive()` is true** — the recording-mode flag is the operator's "I'm recording, suppress dev chrome" signal.
   - At the chip render site (grep for `MOCK-BYPASS` literal: `rg "MOCK-BYPASS" web/`), add a guard:
     ```tsx
     {!isDemoLive() && dataSource === "mock-bypass" && <DataSourceChip kind="mock-bypass" />}
     ```
2. **Audit the other pages** for the same pill — `/approvals`, `/audit`, `/workers`, `/workflows`, `/skills` — per scene-6 F12 the chip should be consistent across surfaces. The fix above propagates if `<DataSourceChip>` is a single component used by all of them. Otherwise add per-page guards.
3. **Optional: add an opt-in dev-only re-enable** via `NEXT_PUBLIC_SHOW_DEV_CHIPS=1` env var so the developer can keep the chip visible in dev while it stays hidden in recording mode.
4. **vitest** at `web/components/skycot/DataSourceChip.test.tsx`:
   - With `isDemoLive=true`, dataSource=`mock-bypass`: assert the chip does NOT render.
   - With `isDemoLive=false`, dataSource=`mock-bypass`: assert the chip DOES render.
   - With `isDemoLive=true`, dataSource=`live`: assert no chip (live mode never shows it).

## Acceptance criteria

- [ ] On `/team` with `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` set, the `MOCK-BYPASS` pill is not visible.
- [ ] On `/team` with `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=0` (or unset), the pill still renders for dev visibility.
- [ ] Same behaviour propagates to `/approvals`, `/audit`, `/workers`, `/workflows`, `/skills`.
- [ ] vitest pins both branches.
- [ ] PR body cites scene-1-and-2 F10.

## May-touch

- `web/components/skycot/DataSourceChip.tsx` (or wherever the chip lives)
- `web/app/(app)/team/page.tsx` (only if the chip is page-local not component)
- `web/components/skycot/DataSourceChip.test.tsx` (NEW or extend)
- `web/lib/data-source.ts` (already exposes `isDemoLive()` and `dataSource` per the MERGED #2432 brief)

## Out of scope

- Renaming `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (covered by `fix-web-mock-bypass-env-var-rename` MERGED #2242)
- Demo banner copy and dismissibility (covered by sibling demo-banner briefs)
- Other dev-mode chrome (Next.js dev-tools badge covered by `fix-spa-nextjs-dev-tools-badge-in-recording` IN_REVIEW #2371)

## Effort

XS — ~10 LOC guard + ~40 LOC tests. ~1 hour.
