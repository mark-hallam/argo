# fix-team-empty-state-gendered-language-her-to-them

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-empty-state-gendered-language-her-to-them`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F15 [P3].

🟢 **P3 — `TeamEmptyState.tsx:27` reads "Pause or adjust **her** anytime — she's your teammate, not your tool." Hardcoded `her` in the generic empty-state. The roster mixes Mariah/Peta/Priya (she), Patrick/Jamal (he), and Nori (ambiguous). Either go gender-neutral or randomize.**

## Root cause

`web/components/skycot/TeamEmptyState.tsx:26-28` hardcodes "her" / "she's" in the empty-state copy. The empty state shows when a tenant has zero workers — so the copy can't predict which gender (if any) the user will choose to hire. Hardcoding "her" is a stale assumption from when the only persona in the runbook was Mariah.

The roster of canonical sample workers includes:
- Mariah (she), Peta (she), Priya (she)
- Patrick (he), Jamal (he)
- Nori (ambiguous / they)

So even after the user hires, the displayed copy may not match the actual hired worker's gender pronoun.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F15 [P3] — "Pause or adjust **her** anytime" — gendered language in empty-state copy

## Fix recipe

1. **Edit `web/components/skycot/TeamEmptyState.tsx:26-28`.** Change:
   - Before: `Pause or adjust **her** anytime — she's your teammate, not your tool.`
   - After: `Pause or adjust **them** anytime — they're your teammate, not your tool.`
2. **Audit for other gendered copy.** `rg '\\bher\\b|\\bshe\\b|\\bhim\\b|\\bhis\\b' web/components/skycot/` and `web/app/` for any other places where the copy assumes a gender. Replace with neutral pronouns unless the copy is specifically about a known persona (e.g. a sample-worker bio).
3. **Vitest pin** at `web/components/skycot/__tests__/TeamEmptyState.test.tsx`: assert the rendered text contains "them" / "they're" and does not contain "her" / "she's".

## Acceptance criteria

- [ ] `TeamEmptyState` renders with gender-neutral pronouns.
- [ ] `rg 'her anytime|she\\'s your teammate' web/` returns 0 matches.
- [ ] Vitest pin asserts the new copy.
- [ ] Other generic-copy surfaces audited; bio/persona-specific copy untouched.

## May-touch

- `web/components/skycot/TeamEmptyState.tsx`
- `web/components/skycot/__tests__/TeamEmptyState.test.tsx` (new or extend)
- Any other generic empty-state with gendered copy (per audit)

## Out of scope

- Sample-worker bio copy (Mariah/Peta/Priya/Patrick/Jamal/Nori) — those are about specific personas and can use their canonical pronouns
- Marketing pages — separate scope
- i18n / localization (separate effort)

## Effort

XS — ~5-10 LOC. ~10-20 min. P3 because it's a small inclusivity nit; the v1.0 demo MP4 may not even show this empty state. Worth fixing because it's a 30-second change and the operator-locked vocab principles favor inclusive copy.
