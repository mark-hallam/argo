# fix-workflows-templates-title-chain-break-and-genesis-h1-studio-framing

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-templates-title-chain-break-and-genesis-h1-studio-framing`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/whole-site-mvp-claims-coverage.md` F13 + F17.

🟡 **P2 — Two small framing defects on adjacent in-app routes. (1) `/workflows/templates` breaks the `· SKYCOT` title chain — the layout exports `title: "Workflow templates"` as a string literal, which Next.js treats as an override that doesn't inherit the parent's `"%s · SKYCOT"` template. Tab reads "Workflow templates" without the SKYCOT suffix. (2) `/genesis` H1 reads "Hire your next teammate." — pure hire-CTA framing — but per operator memory `feedback_genesis_is_ai_teammate_onboarding_studio` (2026-05-25), Genesis is the AI Teammate Onboarding Studio. The H1 should signal "studio", not "hire button".**

## Root cause

**(1) Title chain break.** `web/app/(app)/workflows/templates/layout.tsx:5`:
```ts
export const metadata = {
  title: "Workflow templates",
};
```

When the title is a plain string, Next.js treats it as `title.absolute` — it overrides ALL parents and emits the literal string. To inherit the parent's template (`"%s · SKYCOT"`), the metadata needs to use the `{template, default}` form OR explicitly call out `title.default`:

Correct shape:
```ts
export const metadata = {
  title: { default: "Workflow templates" },  // inherits parent template
};
```

OR more explicitly:
```ts
export const metadata = {
  title: { absolute: "Workflow templates · SKYCOT" },  // bypass template, emit literal
};
```

Either works. The current code accidentally bypasses the template.

**(2) Genesis H1 framing.** `web/app/(app)/genesis/page.tsx` renders `BrandLoudHero` with hero copy "Hire your next teammate." (or similar — verify by reading the component). Per operator memory `feedback_genesis_is_ai_teammate_onboarding_studio`: Genesis is the AI Teammate Onboarding Studio — it takes voice/text role descriptions + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification. It is NOT the hire CTA (that's the onboarding wizard `/onboarding/first-hire`). The H1 should reframe.

Recommended H1 copy:
- Eyebrow: `ONBOARDING STUDIO`
- Hero: `Onboard a new teammate.`
OR
- Eyebrow: `AI TEAMMATE ONBOARDING`
- Hero: `Bring them up to speed.`

Operator picks the final wording.

## Findings this brief closes

- whole-site-mvp-claims-coverage F13 [P1] — `/workflows/templates` STILL breaks the `· SKYCOT` title chain
- whole-site-mvp-claims-coverage F17 [P2] — `/genesis` H1 mismatches operator-clarified studio scope

## Fix recipe

**Part 1 — Workflows templates title:**

1. In `web/app/(app)/workflows/templates/layout.tsx:5`, change to:
   ```ts
   export const metadata = {
     title: { default: "Workflow templates" },
   };
   ```
2. Verify the rendered title is "Workflow templates · SKYCOT" via Playwright DOM check.

**Part 2 — Genesis H1:**

3. In `web/app/(app)/genesis/page.tsx`, locate the `<BrandLoudHero>` props or inline hero text.
4. Update the eyebrow to `ONBOARDING STUDIO` (operator's locked framing).
5. Update the hero copy to `Onboard a new teammate.` (or operator-approved alternative — recommend asking operator to pick from 2-3 options).
6. Verify the rest of the page's studio inputs (HeroInput, PreviewCard, ProgressStrip) remain unchanged.

**Part 3 — Sweep for sibling title-chain breaks.** `rg "title:\s*['\"]" web/app/(app)/**/layout.tsx` to find any other plain-string title literals that bypass the template. Document or fix each.

## Acceptance criteria

- [ ] `/workflows/templates` browser tab reads "Workflow templates · SKYCOT".
- [ ] `/genesis` H1 reads "Onboard a new teammate." (or operator-approved equivalent) with the eyebrow "ONBOARDING STUDIO".
- [ ] No other `(app)/**` layout.tsx exports a plain-string `title` that breaks the template chain (verified by `rg`).
- [ ] Playwright pin asserts both surfaces' titles + H1.

## May-touch

- `web/app/(app)/workflows/templates/layout.tsx` (metadata shape)
- `web/app/(app)/genesis/page.tsx` (hero copy)
- `web/components/skycot/BrandLoudHero.tsx` (no shape change expected; consumer prop change)
- `web/__tests__/title-chain.spec.ts` (Playwright pin)

## Out of scope

- The `/genesis` orphan-discoverability question (covered by `fix-genesis-orphan-discoverability-and-provider-key-503.md`)
- The Genesis page H1 onboarding-studio rename (covered by `fix-genesis-page-h1-onboarding-studio-not-hire.md` — overlaps; coordinate. If that brief already covers Part 2, this brief shrinks to just Part 1).
- The (app) shell default-title sweep (covered by `fix-app-shell-default-metadata-title-leaks-approvals-across-9-pages.md`)

## Effort

XS — ~10-20 LOC across 2 files + 1 pin. ~30 minutes. P2 because both are small but camera-visible during navigation beats.
