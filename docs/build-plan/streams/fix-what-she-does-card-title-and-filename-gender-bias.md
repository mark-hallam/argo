# fix-what-she-does-card-title-and-filename-gender-bias

**Owner**: claude.
**Authored against**: origin/main `74077bf676` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-what-she-does-card-title-and-filename-gender-bias`.
**Source**: ux-audit-2026-05-26-2214 `scene-1-and-2-claim-1-and-2.md` F8 [P1].

🟠 **P1 — `WorkerPersonalityCard.tsx`'s sibling component `WhatSheDoesCard.tsx` hardcodes `<CardTitle>What she does</CardTitle>` AND the filename itself encodes the gender. Rendered on every worker overview tab including Jamal (male persona) — viewer reads "What she does" on his profile during an investor watch. Camera-visible inclusivity own-goal. Sister to the existing pronoun-policy brief but with a distinct fix (card title, not body copy).**

## Root cause

`web/components/skycot/worker/WhatSheDoesCard.tsx:103` hardcodes:

```tsx
<CardTitle>What she does</CardTitle>
```

The file name itself (`WhatSheDoesCard.tsx`) also embeds the bias and ships in repo grep results. The component is rendered on every worker overview tab:

```
/workers/018f67d0-0000-7000-8000-000000000003/overview  (Jamal)
→ Card title: "What she does"
→ Body refers to "Jamal" by name elsewhere on the page
```

Per the operator's plain-language preferences + `feedback_no_theater_in_demo` rule, the title should not assume gender. The locked C2 personality design (`project_worker_detail_design_locked` memory) emphasises "your AI hires are colleagues" — gender-defaulting card titles undermine that frame.

Note: this is DIFFERENT from `fix-worker-pronouns-policy-they-or-explicit-pronoun-field` and `fix-worker-activity-pronoun-neutral-and-dev-chrome-gating` — those briefs cover BODY copy ("she's working" / "her activity") on different surfaces. THIS brief specifically targets the card TITLE + the component FILENAME.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F8 [P1] — Hardcoded gendered copy "What she does" on Overview tab for non-female workers

## Fix recipe

**Path A (recommended, minimal-friction):** rename the title to a gender-neutral default + rename the file:

1. **Rename file**: `WhatSheDoesCard.tsx` → `WhatTheyDoCard.tsx`
   - `git mv web/components/skycot/worker/WhatSheDoesCard.tsx web/components/skycot/worker/WhatTheyDoCard.tsx`
   - `git mv web/components/skycot/worker/__tests__/WhatSheDoesCard.test.tsx` if present

2. **Rename the component + title** at `WhatTheyDoCard.tsx:103`:
   ```tsx
   <CardTitle>What they do</CardTitle>
   ```
   Also rename the exported component `WhatSheDoesCard` → `WhatTheyDoCard`.

3. **Update all imports** — `rg "WhatSheDoesCard" web/` to find usages; update each.

**Path B (operator-decisional follow-up):** thread the worker's pronoun via `SamplePersona.pronouns` and render "What he does" / "What she does" / "What they do" accordingly. Requires the broader pronoun-field refactor in `fix-worker-pronouns-policy-they-or-explicit-pronoun-field`.

Ship Path A now. Path B is the long-term architecture and can land later.

## Acceptance criteria

- [ ] File `web/components/skycot/worker/WhatSheDoesCard.tsx` no longer exists
- [ ] File `web/components/skycot/worker/WhatTheyDoCard.tsx` exists with `<CardTitle>What they do</CardTitle>`
- [ ] All imports of `WhatSheDoesCard` are updated to `WhatTheyDoCard`
- [ ] `rg "What she does\|WhatSheDoesCard" web/` returns no matches
- [ ] `/workers/<jamal-id>/overview` renders "What they do" as the card title
- [ ] `pnpm --dir web exec tsc --noEmit` green
- [ ] `pnpm --dir web exec vitest run components/skycot/worker` green

## May-touch

- `web/components/skycot/worker/WhatSheDoesCard.tsx` (renamed + edited, ~5 LOC change)
- `web/components/skycot/worker/__tests__/WhatSheDoesCard.test.tsx` (renamed if present)
- All callsites with `import { WhatSheDoesCard }` (usually 1-2 files, ~2 LOC each)

## Out of scope

- Body-copy pronouns ("she's working" / "her activity") — covered by `fix-worker-pronouns-policy-they-or-explicit-pronoun-field` + `fix-worker-activity-pronoun-neutral-and-dev-chrome-gating`
- Adding a `pronouns` field to `SamplePersona` (Path B; future refactor)
- The Genesis/Hire wizard pronoun-pick flow (separate concern)

## Effort

~10 LOC change across 2-3 files. ~20 min. P1 because camera-visible during Scene 1 + Scene 2 when the operator drills into any non-female worker.
