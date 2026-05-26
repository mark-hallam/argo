# fix-worker-pronouns-policy-they-or-explicit-pronoun-field

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-pronouns-policy-they-or-explicit-pronoun-field`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-1-and-2-claim-1-and-2.md` F9 [P2].

🟡 **P2 — Peta's overview narration calls her "she" twice ("What she does" / "Peta hasn't picked up Slack or email yet" / "Connect a channel so Peta can respond..."). Component name `WhatSheDoesCard` even leaks the bias. Risks reading as "we hardcoded gender per name" rather than "operator picks pronouns at hire time". Brand-policy decision needed.**

## Root cause

Sample-persona copy at `web/lib/sample-worker-overview.ts` (and possibly `web/lib/sample-personas.ts`) uses gendered pronouns inferred from name. Component name `WhatSheDoesCard.tsx` literally encodes the bias in the file name. There's no `pronouns` field on `SamplePersona`, so the gendered copy is hardcoded per-persona without an operator-controlled override.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F9 [P2] — Worker pronouns inconsistent (Peta = "she" hardcoded)

## Fix recipe

Three options — operator picks; recommend (b):

**Option (a): settle on "they" for all sample personas.** Lowest-friction inclusive default. Edit all `she/he/her/his/him` in `web/lib/sample-worker-overview.ts` to `they/them/their`. Rename `WhatSheDoesCard.tsx` to `WhatTheyDoCard.tsx`.

**Option (b) [recommended]: ship a `pronouns` field on `SamplePersona`** so it's explicit + per-persona + operator-overridable at hire time:
1. Add `pronouns: "she/her" | "he/him" | "they/them" | string` to `SamplePersona` type
2. Default each canonical persona to `"they/them"` unless the operator has explicitly assigned otherwise (Peta = they/them; Mariah = they/them; etc.)
3. Refactor copy from hardcoded "she" to `${pronouns.subject}` / `${pronouns.object}` / `${pronouns.possessive}` via a `formatPronouns()` helper:
   ```ts
   export function formatPronouns(p: string): {subject: string, object: string, possessive: string} {
     switch (p) {
       case "she/her": return {subject: "she", object: "her", possessive: "her"};
       case "he/him": return {subject: "he", object: "him", possessive: "his"};
       default: return {subject: "they", object: "them", possessive: "their"};
     }
   }
   ```
4. Rename `WhatSheDoesCard.tsx` to `WhatTheyDoCard.tsx` (the file name reads as broken software either way; rename for inclusive default)

**Option (c): accept gendered copy as part of the persona design.** Bake explicit pronoun choice per persona, surface in the hire wizard. More work; defer.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F9 [P2] — Worker pronouns inconsistent

## Acceptance criteria

- [ ] No worker-detail page contains the string " she " or " he " or " her " or " his " or " him " (whitespace-bounded) unless the persona has explicitly opted into those pronouns
- [ ] `WhatSheDoesCard.tsx` renamed to `WhatTheyDoCard.tsx` (or equivalent inclusive name)
- [ ] If Option (b): `pronouns` field exists on `SamplePersona` with operator-settable values
- [ ] Component test snapshot covers all 5 canonical personas
- [ ] No regressions in other persona surfaces

## May-touch

- `web/lib/sample-personas.ts` (add `pronouns` field if Option b)
- `web/lib/sample-worker-overview.ts` (refactor copy)
- `web/components/skycot/worker/WhatSheDoesCard.tsx` → `WhatTheyDoCard.tsx` (rename + imports)
- `web/lib/util/pronouns.ts` (NEW helper)
- Any sibling components consuming gendered copy

## Out of scope

- Pronoun-picker UI in the hire wizard (operator decision; v1.1 polish)
- Renaming any of the actual persona names
- Backend / Genesis prompt template adjustment

## Effort

Option (a): ~30 LOC, ~1 hour. Option (b): ~150-250 LOC + tests, ~3-4 hours. P2 polish; brand-policy decision needed before implementation.
