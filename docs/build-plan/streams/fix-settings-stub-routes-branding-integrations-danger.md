# fix-settings-stub-routes-branding-integrations-danger

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-stub-routes-branding-integrations-danger`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F12 (P1).

🟠 **P1 — `/settings/branding`, `/settings/integrations`, and `/settings/danger` all return 404 even though the operator's mental model and audit-brief scope expect them. The Settings layout DOES render a "Danger zone" copy block as a static aside on every sub-page, but no `/settings/danger` route exists. The closest analog to "integrations" is `/settings/connectors`, but the operator's mental model expects both URLs to work.**

## Root cause

`web/components/skycot/settings/SettingsTabs.tsx:31-71` defines the canonical Settings tabs. The current set omits `branding`, `integrations`, and `danger` entirely. The locked Settings design (`project_settings_page_design_locked` memo) is D top-tabs with sub-tabs — `appearance` lives under `/settings/workspace`, and "Danger zone" is a right-aligned section, not its own route.

But the operator's audit brief lists `branding`, `integrations`, `danger` as expected — meaning users (and likely external docs / quickstart links) are typing those URLs. The 404 wastes a navigation attempt; the right move is either to:

1. Add redirect routes: `/settings/branding → /settings/workspace#appearance`, `/settings/integrations → /settings/connectors`, `/settings/danger → /settings/workspace#danger-zone`.
2. Add stub pages with a `<Redirect>` component and a friendly "moved to X" message.

Option 1 is cleaner and faster; Option 2 helps when external docs link the old URL.

This is not a demo blocker (Scene 6 doesn't go here) but it's a polish gap that any post-demo trial user will hit.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F12 [P1] — `/settings/{branding,integrations,danger}` return 404

## Fix recipe

1. Decide canonical destinations:
   - `branding` → `/settings/workspace` (the appearance + workspace identity section)
   - `integrations` → `/settings/connectors` (the data connectors page)
   - `danger` → `/settings/workspace` with `#danger-zone` anchor (Danger zone is right-aligned on the workspace tab)
2. Implement redirects in `web/middleware.ts` or `web/next.config.ts`:
   ```ts
   { source: "/settings/branding", destination: "/settings/workspace#appearance", permanent: false },
   { source: "/settings/integrations", destination: "/settings/connectors", permanent: false },
   { source: "/settings/danger", destination: "/settings/workspace#danger-zone", permanent: false },
   ```
3. Verify each redirect: `curl -sI http://127.0.0.1:3000/settings/branding` returns 307.
4. Optional: add a `data-testid="route-redirect-source"` to allow playwright to assert the redirect lands at the right destination.

## Acceptance criteria

- [ ] `curl -sI http://127.0.0.1:3000/settings/branding` returns 307 → `/settings/workspace#appearance`.
- [ ] `curl -sI http://127.0.0.1:3000/settings/integrations` returns 307 → `/settings/connectors`.
- [ ] `curl -sI http://127.0.0.1:3000/settings/danger` returns 307 → `/settings/workspace#danger-zone`.
- [ ] Playwright smoke verifies the destinations exist + scroll to the right anchor.
- [ ] No regression to the existing Settings tabs.

## May-touch

- `web/middleware.ts` OR `web/next.config.ts` (~10 LOC redirects)
- `web/tests/e2e/settings-redirects.spec.ts` (NEW — ~30 LOC smoke)
- `docs/skycot/operator/quickstart.md` (if quickstart references the old URLs — verify)

## Out of scope

- Building a separate `/settings/branding` page (the locked D design says appearance lives under workspace).
- The actual content of `/settings/workspace` or `/settings/connectors` (unchanged).
- Adding new Settings sub-tabs.

## Effort

XS — ~10 LOC redirects + a smoke test. ~30 minutes. P1 because external docs / quickstart guides likely link these URLs (or will) and the 404 is a credibility hit for any operator typing them.
