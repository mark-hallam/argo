# fix-settings-page-implement-d-top-tabs-design

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-page-implement-d-top-tabs-design`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) findings F4 + F12 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟡 **P1 — `/settings` index does not implement the locked D-top-tabs design. Sub-pages have no shared tab strip. Three cards say "Coming soon" which reads as half-finished.**

## Root cause

Memory `project_settings_page_design_locked` calls for "D top-tabs (horizontal tabs + sub-tabs + 2-col content with aside cards), Danger zone right-aligned in tab row".

Current `/settings/page.tsx` ships a 2-column card-grid landing page (Workspace / Team & roles / Notifications / Security / Provider keys / API keys / Notification channels / Knowledge / Connectors / Billing / Data export). When you drill into any sub-route (e.g. `/settings/security`, `/settings/connectors`), there are no horizontal tabs at the top connecting the sub-pages — each is a standalone surface with no parent-tab navigation.

Three cards say "Coming soon" (Workspace, Notifications, Provider keys) — those last especially is a sharp edge because `Provider keys` is the v1-locked surface for managing Anthropic keys per `crates/argo-config/src/providers.rs`. Demo doesn't navigate here, but for an investor who clicks Settings as a sanity check, three "coming soon" labels in eleven cells read as half-finished.

## Findings this brief closes

- whole-site-mvp-claims-coverage F4 [P1] — `/settings` index does not implement the locked D top-tabs design
- whole-site-mvp-claims-coverage F12 [P2] — `/settings` "Coming soon" badges on core surfaces (Workspace / Notifications / Provider keys)

## Fix recipe

1. **Implement the shared layout** at `web/app/(app)/settings/layout.tsx`:
   - Horizontal tab strip across the top: Workspace / Team / Notifications / Security / Providers / API keys / Channels / Knowledge / Connectors / Billing / Data export
   - "Danger zone" right-aligned in the tab row
   - 2-col content area (main + aside cards)
   - Active tab reflects current sub-route
   - Brand-restrained register (Inter only, no Audiowide on this page) per the locked memory

2. **Move sub-pages into the layout's content slot**. Existing `/settings/security/page.tsx`, `/settings/connectors/page.tsx`, etc. become content blocks rendered within the tab layout.

3. **Drop the card-grid landing page**. `/settings` (root) now renders the active default tab — pick Workspace as the default tab (per the locked memory if it specifies) OR redirect `/settings` → `/settings/workspace`.

4. **Handle "Coming soon" cards** — pick ONE per card:
   - (a) Implement the card content (e.g. Provider keys: render a list of installed keys + add/remove flow tied to the existing `crates/argo-config/src/providers.rs` data).
   - (b) Drop the card entirely until the surface is implemented.
   - Do NOT keep "Coming soon" badges in v1.

5. **Pin** with a Playwright spec at `web/tests/settings-layout-tabs.spec.ts`:
   - Navigate `/settings/security`; assert the tab strip is present and "Security" tab is active.
   - Navigate `/settings/connectors`; assert tab strip is present and "Connectors" tab is active.
   - Assert no "Coming soon" badges anywhere in the rendered DOM.

## Acceptance criteria

- [ ] `/settings` and all sub-routes share a horizontal tab strip per the locked D-top-tabs design.
- [ ] Active tab reflects the URL.
- [ ] Danger zone tab is right-aligned in the tab row.
- [ ] No "Coming soon" badges in any settings card or tab.
- [ ] Playwright spec passes.
- [ ] Brand-restrained register: Inter throughout (no Audiowide) per the locked memory.

## May-touch

- `web/app/(app)/settings/layout.tsx`
- `web/app/(app)/settings/page.tsx` (drop or replace with the active default tab content)
- `web/app/(app)/settings/<sub>/page.tsx` (remove standalone shells; render only content)
- `web/components/skycot/settings/SettingsTabs.tsx` (NEW)
- `web/tests/settings-layout-tabs.spec.ts` (NEW)

## Out of scope

- `/settings/members` hydration mismatch (separate brief `fix-settings-members-hydration-mismatch-unstyled`)
- `/settings/security` SAML status "Failed to fetch" (separate brief `fix-settings-security-saml-status-failed-to-fetch`)
- Building Provider keys CRUD (separate brief; could ship as follow-up)
- Wider hydration issues (separate brief `fix-spa-hydration-warnings-theme-and-motion-providers`)

## Effort

~200-400 LOC (new layout + tabs + extracting sub-page content). ~4-6 hours.
