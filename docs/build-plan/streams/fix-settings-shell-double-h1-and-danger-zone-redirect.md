# fix-settings-shell-double-h1-and-danger-zone-redirect

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-shell-double-h1-and-danger-zone-redirect`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/whole-site-mvp-claims-coverage.md` F13 [P1] + F14 [P1].

🟠 **P1 — Every settings sub-page renders TWO `<h1>` elements: one from the settings shell ("Settings") and one from the sub-page ("API keys" / "Security" / etc.). Breaks screen-reader heading hierarchy and SEO. Separately, `/settings/danger-zone` returns 404 — the Danger zone is an anchor `#danger-zone` on `/settings/security`, not a separate route; a typed URL produces a 404 dead-end.**

## Root cause

**Double h1**: `web/app/(app)/settings/layout.tsx:18` renders `<h1>Settings</h1>` at the top of the shell. Each sub-page then renders its own `<h1>` (e.g. `<h1 class="ak-title">API keys</h1>` in `web/app/(app)/settings/api-keys/page.tsx`). Result: every page has two h1s. Verified by `curl http://127.0.0.1:3000/settings/api-keys | grep -oE "<h1[^>]*>[^<]+"` returning two matches.

**Danger zone 404**: the settings layout includes the Danger zone as an `#danger-zone` anchor at `web/app/(app)/settings/layout.tsx:46` on the Security sub-page. There is no `/settings/danger-zone` route. A user / search-engine / documentation reference that types the URL gets a 404.

## Findings this brief closes

- whole-site-mvp-claims-coverage F13 [P1] — `/settings/danger-zone` 404 (anchor, not a route)
- whole-site-mvp-claims-coverage F14 [P1] — Every settings sub-page renders two h1 elements

## Fix recipe

### Double h1 fix

1. **Decision point** — pick one:
   - (a) Settings shell keeps `<h1>Settings</h1>`; each sub-page downgrades to `<h2>` for its page-title.
   - (b) Settings shell uses `<div class="settings-shell-title">Settings</div>` (visual h1 styling without semantic h1); each sub-page keeps its `<h1>`.

   Recommended: (b) — each settings page is its own document; "Settings" is shell chrome, not the document title. The sub-page's title (API keys / Security / etc.) IS the document title.

2. **Update the shell** at `web/app/(app)/settings/layout.tsx:18`:
   ```tsx
   <div role="banner" aria-label="Settings" className="settings-shell-title">
     Settings
   </div>
   ```
   Keep visual styling identical to today's `<h1>`.

3. **Verify each sub-page's `<h1>`** is the SOLE h1 on the page. Sub-pages: workspace, members, notifications, security, providers, api-keys, notify-channels, knowledge, connectors, billing, data-export. Audit each to ensure exactly one h1.

### Danger zone 404 fix

4. **Add a Next.js redirect** at `web/next.config.ts`:
   ```ts
   async redirects() {
     return [
       { source: "/settings/danger-zone", destination: "/settings/security#danger-zone", permanent: false },
     ];
   }
   ```

5. **Optional**: surface "Danger zone" as a distinct row in the settings tabs (with `#danger-zone` href) if the design-lock allows. Today it's anchor-only.

### Tests

6. **Add a Playwright spec** `web/playwright/settings/heading-hierarchy-and-redirects.spec.ts`:
   - For each settings sub-page, assert `page.locator("h1").count()` returns exactly 1.
   - Visit `/settings/danger-zone`; assert the final URL is `/settings/security` with the hash `#danger-zone`.

## Acceptance criteria

- [ ] Every settings sub-page has exactly one `<h1>` element.
- [ ] Screen-reader heading hierarchy reads cleanly (verify with macOS VoiceOver "Headings menu" or the Chrome accessibility devtools tree).
- [ ] `/settings/danger-zone` redirects to `/settings/security#danger-zone`.
- [ ] New Playwright spec passes.
- [ ] No visual regression on the settings shell layout (D top-tabs lock preserved).

## May-touch

- `web/app/(app)/settings/layout.tsx` (shell h1 → div)
- All 11 settings sub-page `page.tsx` files (verify single h1 each)
- `web/next.config.ts` (redirect rule)
- `web/playwright/settings/heading-hierarchy-and-redirects.spec.ts` (NEW)

## Out of scope

- Settings sub-page title suffixes (covered by `fix-settings-sub-pages-title-suffix-consistency`).
- D top-tabs design implementation (covered by `fix-settings-page-implement-d-top-tabs-design`).
- Members hydration mismatch (covered by `fix-settings-members-hydration-mismatch-unstyled`).
- Security SAML status (covered by `fix-settings-security-saml-status-failed-to-fetch`).

## Effort

~50-100 LOC + 1 spec. ~2-4 hours. P1 — accessibility regression + crawler-trap that any external bookmark could hit.
