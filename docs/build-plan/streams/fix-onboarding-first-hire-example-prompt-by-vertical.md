# fix-onboarding-first-hire-example-prompt-by-vertical

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-first-hire-example-prompt-by-vertical`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md` F12 [P2].

🟡 **P2 — First-hire step (step 5) hardcodes `EXAMPLE_PROMPT = "A marketing intern named Maya..."` as the textarea placeholder. The operator has already picked a vertical in step 4 (e.g. GTM Engineer). The placeholder example contradicts the picked template, and Genesis dispatches on prompt-text archetype words ("marketing intern") rather than the picked vertical — so typing the placeholder example gets a Marketing Intern plan despite picking "GTM Engineer".**

## Root cause

`web/app/(setup)/onboarding/first-hire/page.tsx:14-15`:
```ts
const EXAMPLE_PROMPT = "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans.";
```

Hardcoded; doesn't read the picked `draft.verticalId`. Combined with `crates/argo-cli/src/handlers/gateway/routes_onboarding_genesis.rs:75-95` which dispatches Genesis on the prompt text (not on the picked vertical id), the result is: pick "GTM Engineer" → see "marketing intern" placeholder → type the placeholder → get a marketing-intern plan.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F12 [P2] — First-hire prompt placeholder seeds a vertical that contradicts the picked vertical

## Fix recipe

1. **Parameterise the placeholder by vertical**. Add a lookup table at `web/lib/onboarding/example-prompts.ts`:
   ```ts
   export const EXAMPLE_PROMPTS: Record<string, string> = {
     "marketing-intern": "Maya is a marketing intern who drafts blog posts and runs weekly competitive scans.",
     "gtm-engineer": "Mariah is a GTM engineer who researches enterprise prospects and books demo calls.",
     "people-ops": "Peta coordinates the team — sprint kickoffs, status reports, async standups.",
     "support": "Jamal triages tier-1 tickets, drafts response templates, and escalates refunds over $500.",
     "sales": "Priya is an enterprise AE who qualifies inbound leads and prepares discovery calls.",
     "finance": "Nori handles AP invoices, vendor reconciliation, and month-end close checks.",
     // ... cover all verticals shipped in step 4
   };
   ```

2. **In `web/app/(setup)/onboarding/first-hire/page.tsx`**:
   - Read `draft.verticalId` from wizard state
   - `const examplePrompt = EXAMPLE_PROMPTS[draft.verticalId] ?? EXAMPLE_PROMPTS["marketing-intern"];`
   - Pass to the textarea's `placeholder` prop

3. **Optionally also pre-fill** the textarea with `examplePrompt` if the operator hasn't started typing — saves them a step and makes Genesis dispatch consistent. Provide a "Start over" button to clear if they want a clean slate.

4. **Add a Playwright spec** at `web/playwright/onboarding/first-hire-prompt-matches-vertical.spec.ts`:
   - Walk through wizard, pick GTM Engineer in step 4
   - Land on step 5
   - Assert the textarea placeholder contains "GTM engineer" or "Mariah" (vertical-appropriate)
   - Repeat for 2 other verticals

## Acceptance criteria

- [ ] Placeholder text in first-hire step matches the picked vertical from step 4
- [ ] Picking GTM Engineer in step 4 → placeholder mentions GTM engineer / Mariah / similar
- [ ] Picking Marketing Intern in step 4 → placeholder mentions marketing intern / Maya / similar
- [ ] New Playwright spec passes

## May-touch

- `web/lib/onboarding/example-prompts.ts` (NEW)
- `web/app/(setup)/onboarding/first-hire/page.tsx:14-15`
- `web/playwright/onboarding/first-hire-prompt-matches-vertical.spec.ts` (NEW)

## Out of scope

- The Genesis dispatch-on-prompt-text bug — Genesis SHOULD respect the picked verticalId hint; separate brief if needed
- Name parser (covered by `fix-onboarding-first-hire-name-parser-prefers-named-pattern`)
- Genesis invocation itself (covered by `fix-onboarding-first-hire-invoke-genesis-plan`)

## Effort

~50-100 LOC + 1 spec. ~1-2 hours. P2 polish but eliminates a foot-gun where Genesis dispatches against the wrong archetype.
