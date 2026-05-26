# fix-peta-personality-trait-level-lookup-coverage

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-peta-personality-trait-level-lookup-coverage`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-1-and-2-claim-1-and-2.md` F4 [P1].

🟠 **P1 — Peta is in the canonical 5-card roster (#2367 fix) but missing from `TRAITS_BY_NAME` + `LEVEL_BY_NAME` + `PERSONALITIES` lookup tables. Her card on `/team` has NO trait chips (other 4 do). Her detail page falls through to generic boilerplate ("Reliable / Curious / Helpful") instead of her actual traits ("Cadence-true / Follow-through / Calm escalator / Context keeper"). Sibling-table-sprawl regression — the same bug pattern that produced this initially.**

## Root cause

The locked C2-holographic-showcase design pattern depends on 4 parallel name-keyed lookup tables:
- `TRAITS_BY_NAME` at `web/components/skycot/TeammateCard.tsx:51-67` — drives the trait chip strip
- `LEVEL_BY_NAME` at the same file — hand-tuned level numbers (5, 9, 12, 15)
- `PERSONALITIES` (or `PERSONALITY_BY_NAME`) at `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111` — drives the "HOW <NAME> WORKS" card
- `DEFAULT_PERSONALITY` at `WorkerPersonalityCard.tsx:98` — fallback when name not in the map

PR #2367 added Peta to `SAMPLE_PERSONAS` + `SAMPLE_PERSONA_IDS` (the roster) but missed the 3 personality tables. So:
- `/team` deck: Peta's card renders but `TRAITS_BY_NAME["Peta"] ?? []` returns empty → no trait chip strip
- `/team` deck: Peta's level number defaults to `hashLevel("Peta") = 18` (unhandled) instead of curated 5/9/12/15
- `/workers/{peta-id}/overview`: Personality card header says "HOW PETA WORKS" then shows generic "Reliable / Curious / Helpful"

Peta's actual personality data DOES exist at `web/lib/sample-personas.ts:136` — it's just not consumed by the 3 lookup tables. Sibling-table sprawl: 4 parallel maps to maintain when adding a persona.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F4 [P1] — Peta missing from TRAITS_BY_NAME + LEVEL_BY_NAME + PERSONALITIES

## Fix recipe

**Two options** — pick (b) to also kill the sibling-table sprawl that caused the bug:

### Option A (quick): add Peta to all 3 maps
Trivial 9-line edit. Add `Peta: [...traits]` to each of the 3 maps. Risk: same bug recurs on the next persona added.

### Option B (recommended): refactor all 3 maps to consume `sample-personas.ts`

1. **Extend `SamplePersona` type** at `web/lib/sample-personas.ts` to carry the union of fields the 3 maps need:
   ```ts
   export type SamplePersona = {
     id: string;
     name: string;
     traits: string[];          // already exists
     level: number;             // NEW (curate per persona)
     personality: {             // NEW (the WorkerPersonalityCard payload)
       summary: string;
       greatAt: string[];
       signatureBehaviors: string[];
       escalateWhen: string[];
     };
     // ... existing fields
   };
   ```

2. **Populate the new fields** for each of the 5 canonical personas (Mariah, Peta, Priya, Jamal, Nori), copying values from the existing 3 maps as starting points + filling in Peta's from her existing copy at `sample-personas.ts:136`.

3. **Replace the 3 maps with helper functions** that look up by workerId (not by name) — gracefully fallback to defaults when persona not found:
   ```ts
   // TeammateCard.tsx
   const persona = findSamplePersona(worker.id);
   const traits = persona?.traits ?? [];
   const level = persona?.level ?? hashLevel(worker.name);
   
   // WorkerPersonalityCard.tsx
   const persona = findSamplePersona(worker.id);
   const personality = persona?.personality ?? DEFAULT_PERSONALITY;
   ```

4. **Delete `TRAITS_BY_NAME`, `LEVEL_BY_NAME`, `PERSONALITIES` maps** entirely.

5. **Add a unit test** at `web/lib/sample-personas.test.ts`:
   ```ts
   it("every canonical persona has personality + traits + level", () => {
     for (const p of SAMPLE_PERSONAS) {
       expect(p.traits).toHaveLength(4);
       expect(p.level).toBeGreaterThan(0);
       expect(p.personality.greatAt.length).toBeGreaterThan(0);
     }
   });
   ```

## Acceptance criteria

- [ ] Peta's card on `/team` shows 4 trait chips
- [ ] Peta's level number is curated (not 18)
- [ ] Peta's detail page personality card shows her actual personality (Cadence-true / Follow-through / Calm escalator / Context keeper), not the generic boilerplate
- [ ] The 3 maps are gone (Option B) OR Peta is added to all 3 (Option A)
- [ ] Unit test covers all canonical personas have the required fields
- [ ] No regressions for other 4 personas

## May-touch

- `web/lib/sample-personas.ts` (extend type + populate fields)
- `web/components/skycot/TeammateCard.tsx:51-67` (replace with helper)
- `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111` (replace with helper)
- `web/lib/sample-personas.test.ts` (NEW unit test)

## Out of scope

- The actual personality copy choices for any worker (operator-decisional; just preserve what's there)
- The C2-holographic visual treatment (locked design; unchanged)
- Pronoun policy (covered by separate brief)

## Effort

Option A: ~10-20 LOC, ~30 min. Option B: ~150-300 LOC + 1 test, ~3-5 hours. Strongly recommend Option B to prevent recurrence.
