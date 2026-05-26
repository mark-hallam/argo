# fix-team-page-mock-offline-chip-suppress-during-recording

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-mock-offline-chip-suppress-during-recording`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` F7 [P2].

🟡 **P2 — Inside the heading "Team" the badge "MOCK-OFFLINE" renders as a sibling chip. Fine for dev work but appears on screen for the entire team page during investor recording. The "Demo mode — showing sample data" top banner already communicates the state; the duplicate MOCK-OFFLINE chip reads as broken software.**

## Root cause

When the SPA falls through to `SAMPLE_TEAM` because the gateway 404s, the `data-source` tag is set to `mock-offline` and rendered as a visible chip in the page heading. This is developer-side diagnostic info; investor-facing pages should not show it.

## Findings this brief closes

- chrome-live-walk-and-playwright F7 [P2] — Demo-mode "MOCK-OFFLINE" chip visible inside Team header

## Fix recipe

1. **Locate the MOCK-OFFLINE chip render** at `web/components/skycot/TeamPageHeader.tsx` (or wherever the chip lives — grep for `MOCK-OFFLINE`).

2. **Gate the chip on a dev-only flag**:
   ```tsx
   {process.env.NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP === "1" && (
     <DataSourceChip source={dataSource} />
   )}
   ```

3. **Move the diagnostic** to a `data-source` HTML attribute on the page root so it's still inspectable in DevTools but not visible to a user:
   ```tsx
   <main data-source={dataSource}>...</main>
   ```

4. **Audit other pages** for the same chip pattern. Grep `web/` for `MOCK-OFFLINE` and `data-source` chip renders. Apply the same gating consistently.

5. **Add a Playwright spec** at `web/playwright/no-mock-offline-chip-in-investor-recording.spec.ts`:
   - With `NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP` unset
   - Load `/team`
   - Assert no visible element contains text "MOCK-OFFLINE" or "mock-offline"
   - Repeat for `/audit`, `/approvals`, `/skills`, `/workflows`

## Acceptance criteria

- [ ] MOCK-OFFLINE chip not visible on /team by default
- [ ] MOCK-OFFLINE chip not visible on /audit, /approvals, /skills, /workflows
- [ ] `NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP=1` re-enables for developer use
- [ ] `data-source` attribute still readable in DevTools
- [ ] New Playwright spec passes

## May-touch

- `web/components/skycot/TeamPageHeader.tsx` (or similar)
- Any page-header / shell component that renders the chip
- `web/playwright/no-mock-offline-chip-in-investor-recording.spec.ts` (NEW)

## Out of scope

- The underlying fallback path that produces the mock-offline source (covered by other briefs)
- The DemoBanner top-of-page strip (separate concern; already dismissible)

## Effort

~30-80 LOC + 1 spec. ~1-2 hours. P2 polish; visible on every team-page take.
