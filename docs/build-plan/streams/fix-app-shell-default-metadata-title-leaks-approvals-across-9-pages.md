# fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/whole-site-mvp-claims-coverage.md` F4.

🟡 **P1 — `web/app/(app)/layout.tsx:18-22` sets the metadata template's default to "Approvals". Any in-app page without its own `export const metadata` inherits that default. 9 confirmed leak surfaces today: `/brand`, `/brand/components`, `/retro`, `/genesis`, `/orchestration`, `/verticals`, `/support/inbox`, `/billing`, `/billing/subscribe`. Worst case: `/genesis` (an investor moment per memory `project_pitch_agent_v1_demo`) reads "Approvals · SKYCOT" in the tab during a recording. The fix is a 1-LOC default change plus 9 per-page metadata exports.**

## Root cause

`web/app/(app)/layout.tsx:18-22`:
```ts
export const metadata: Metadata = {
  title: {
    default: "Approvals",
    template: "%s · SKYCOT",
  },
};
```

The `default` value of "Approvals" was set when the homepage `/` happened to be the approvals dashboard. The dual-surface question (`/` vs `/approvals`) is being tracked separately (`fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity.md`). But until that resolves, the default leaks to every page lacking its own metadata export.

Verified leak list (9 confirmed):
- `/brand` — no `export const metadata` in `web/app/(app)/brand/page.tsx`
- `/brand/components` — no metadata
- `/retro` — no metadata
- `/genesis` — no metadata (the investor surface)
- `/orchestration` — no metadata
- `/verticals` — no metadata
- `/support/inbox` — no metadata
- `/billing` — no metadata
- `/billing/subscribe` — no metadata (separate brief `fix-billing-subscribe-missing-h1-and-metadata.md` covers the H1 + metadata for this one)

The browser tab is camera-readable during any tab-switch beat in the recording. A tab reading "Approvals · SKYCOT" on a Genesis page contradicts the page's own H1.

## Findings this brief closes

- whole-site-mvp-claims-coverage F4 [P1] — `(app)/layout.tsx:20` default title "Approvals" leaks across 9+ in-app pages

## Fix recipe

**Part 1 — Change the default to a neutral SKYCOT title.** In `web/app/(app)/layout.tsx:18-22`:
```ts
export const metadata: Metadata = {
  title: {
    default: "SKYCOT",
    template: "%s · SKYCOT",
  },
};
```
Now any page without its own metadata reads "SKYCOT" — neutral, brand-correct, no false "Approvals" framing.

**Part 2 — Add `export const metadata` to the 9 orphan pages.** Each page gets a 1-line metadata export at the top:

```ts
export const metadata = { title: "Brand" };          // /brand
export const metadata = { title: "Brand components" }; // /brand/components
export const metadata = { title: "Retro" };          // /retro
export const metadata = { title: "Onboarding studio" }; // /genesis (per memory: studio framing)
export const metadata = { title: "Orchestration" };  // /orchestration
export const metadata = { title: "Verticals" };      // /verticals
export const metadata = { title: "Support inbox" };  // /support/inbox
export const metadata = { title: "Billing" };        // /billing
// /billing/subscribe — covered by fix-billing-subscribe-missing-h1-and-metadata
```

Each becomes "<page> · SKYCOT" via the existing template wrap.

**Part 3 — Verify approvals dashboard still reads "Approvals · SKYCOT".** Since the dashboard lives at `/` (per F5 from the audit), the approvals page itself needs to declare its own title now that the default isn't doing it. Add `export const metadata = { title: "Approvals" }` to `web/app/(app)/page.tsx`.

**Part 4 — Lint pin.** Add an ESLint rule or a CI check that fails if a new `web/app/(app)/<path>/page.tsx` (or its layout) lacks an `export const metadata` at module scope. Prevent regression. If a lint pin is out of scope, a Playwright pin that walks all in-app routes and asserts the title is the per-page value works.

## Acceptance criteria

- [ ] `web/app/(app)/layout.tsx` default title is "SKYCOT" (not "Approvals").
- [ ] Every page in the 9-leak list above has its own `export const metadata` at module scope.
- [ ] Playwright walk asserts each of those 9 routes shows "<page> · SKYCOT" in the document title.
- [ ] `/` still reads "Approvals · SKYCOT" (page-level metadata added to compensate).
- [ ] Lint pin OR Playwright pin prevents new orphan pages from regressing.

## May-touch

- `web/app/(app)/layout.tsx` (default change)
- `web/app/(app)/page.tsx` (add Approvals metadata)
- `web/app/(app)/brand/page.tsx` + `web/app/(app)/brand/components/page.tsx`
- `web/app/(app)/retro/page.tsx`
- `web/app/(app)/genesis/page.tsx`
- `web/app/(app)/orchestration/page.tsx`
- `web/app/(app)/verticals/page.tsx`
- `web/app/(app)/support/inbox/page.tsx`
- `web/app/(app)/billing/page.tsx` (NOT `/billing/subscribe` — covered separately)
- `web/__tests__/route-metadata.spec.ts` OR `.eslintrc` lint rule

## Out of scope

- `/billing/subscribe` H1 + metadata (covered by `fix-billing-subscribe-missing-h1-and-metadata.md`)
- `/settings/*` 7 missing metadata (covered by `fix-settings-sub-pages-add-missing-metadata`)
- Marketing page metadata gaps (separate scope)
- The `/` vs `/approvals` dual-surface decision (covered by `fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity.md`)

## Effort

S — ~30 LOC across 10 files + 1 pin. ~1-2 hours. P1 because the demo recording reads the tab bar at every navigation.
