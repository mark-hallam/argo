# fix-billing-subscribe-missing-h1-and-metadata

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-billing-subscribe-missing-h1-and-metadata`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/whole-site-mvp-claims-coverage.md` F12.

🟡 **P1 — `/billing/subscribe` has ZERO `<h1>` elements and no metadata export. Browser tab reads "Approvals · SKYCOT" (the (app) layout default leak). Page is a 61 KB pricing tier picker with Free / Pro / Enterprise tiles but no top-level heading. Accessibility violation + demo trust signal violation.**

## Root cause

`web/app/(app)/billing/subscribe/page.tsx` ships:
- No `<h1>` anywhere in the DOM (verified via HTML inspection).
- No `export const metadata = {...}` declaration, so the title falls through to the (app) layout default "Approvals".

This is a sibling defect to the broader title-leak issue covered by `fix-spa-page-titles-metadata-leak-default-approvals.md`, but the missing `<h1>` is additional — it's an accessibility regression (screen-reader users cannot orient via top-of-document heading) AND a demo trust signal (the camera has nothing to read aloud).

Compounding: `web/app/(app)/layout.tsx:20` default title `"Approvals"` is the leak source. That default needs to be neutral ("SKYCOT") AND every (app) page must declare its own metadata. This brief is the subscribe-specific fix; the broader fix is the sibling brief.

## Findings this brief closes

- whole-site-mvp-claims-coverage F12 [P1] — `/billing/subscribe` has no `<h1>` at all + missing metadata leaks "Approvals" title

## Fix recipe

1. **Add `<h1>`** at the top of the subscribe page:
   ```tsx
   <h1 className="skycot-page-title">Choose a plan</h1>
   ```
   Or whatever locked-design typography variant matches the existing /billing page register. Suggested copy: "Choose a plan" / "Subscribe to SKYCOT" / "Pick the plan that fits your team." Default to "Choose a plan" — terse + clear.

2. **Add metadata export**:
   ```tsx
   export const metadata = {
     title: "Subscribe",
     description: "Pick a SKYCOT plan that fits your team.",
   };
   ```
   The (app) layout's `template: "%s · SKYCOT"` will render as "Subscribe · SKYCOT" in the tab.

3. **Verify** in DevTools: tab reads "Subscribe · SKYCOT"; one and only one `<h1>` exists on the page.

4. **Vitest / Playwright pin**: snapshot the `<h1>` count on `/billing/subscribe`; assert metadata title is "Subscribe".

5. **CI guard**: add an a11y lint that flags any `web/app/(app)/**/page.tsx` rendering content without exactly one `<h1>` in the SSR output. (Tracker brief — file separately if too broad for this brief.)

## Acceptance criteria

- [ ] `/billing/subscribe` HTML has exactly one `<h1>` element.
- [ ] Browser tab reads "Subscribe · SKYCOT", not "Approvals · SKYCOT".
- [ ] Page exports `metadata` with `title: "Subscribe"`.
- [ ] Vitest / Playwright pin passes.

## May-touch

- `web/app/(app)/billing/subscribe/page.tsx` (add h1 + metadata)
- Optionally `web/app/(app)/billing/subscribe/layout.tsx` if metadata-by-layout is preferred
- Test pin

## Out of scope

- Broader title-leak across 8+ pages (covered by `fix-spa-page-titles-metadata-leak-default-approvals.md`)
- /billing page H1 (separate — likely already exists)
- Tier card visual design (locked per pricing direction)
- Settings sub-pages metadata (covered by `fix-settings-sub-pages-title-suffix-consistency.md`)

## Effort

XS — ~10-20 LOC + 1 test. ~15-30 min.
