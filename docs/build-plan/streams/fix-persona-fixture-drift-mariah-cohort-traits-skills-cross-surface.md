# fix-persona-fixture-drift-mariah-cohort-traits-skills-cross-surface

**Owner**: claude.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-persona-fixture-drift-mariah-cohort-traits-skills-cross-surface`.
**Source**: ux-e2e-audit-T2309MEL `scene-1-and-2-claim-1-and-2.md` F7 [P1] + F11 [P2].

🟠 **P1 — Mariah's cohort/persona/skills metadata DISAGREES between `/team` and `/workers/<mariah-id>`. On `/team` her card cohort badge says **MARKETPLACE**; on the detail page hero it says **Operations · Procurement**. Nori's TRAITS differ between surfaces ("Creative · On-brand · Concise · Curious" on `/team` vs. "Reliable · Curious · Helpful" on detail). And Mariah's Skills tab lists `rfp-response-drafter v4.0.0` + `renewal-checklist-runner v3.1.2` + `contract-redline-suggester v1.0.0` — professional-services / B2B-procurement skills, not e-commerce listing skills. The locked C2 personality design promises persona attributes that survive both surfaces.**

## Root cause

The 5 canonical personas (Mariah / Peta / Priya / Jamal / Nori) live in MULTIPLE fixture files, each authored independently against an evolving sense of "what does this persona do?":

1. **`web/lib/sample-team.ts`** — drives the `/team` card deck (cohort badge, top-line description)
2. **`web/lib/sample-personas.ts`** — meant to be the persona spine per its file header (traits, level, role)
3. **`web/components/skycot/worker/WorkerPersonalityCard.tsx`** PERSONALITIES map — drives the `/workers/<id>/overview` "HOW {NAME} WORKS" section (traits + greatAt + escalateWhen + signatureBehaviors)
4. **`web/components/skycot/TeammateCard.tsx`** TRAITS_BY_NAME + LEVEL_BY_NAME — drives card chips and level numbers
5. **`web/lib/sample-worker-skills.ts`** (or equivalent) — drives the Skills tab content

Each file has a slightly different copy of each persona. Specifically observed:

- Mariah: `sample-team.ts` cohort=`marketplace` → label "MARKETPLACE"; `sample-personas.ts` (and the detail hero) → "Operations · Procurement"
- Nori: `TeammateCard.TRAITS_BY_NAME["Nori"]` = [Creative / On-brand / Concise / Curious]; `WorkerPersonalityCard.PERSONALITIES` has no Nori entry → DEFAULT_PERSONALITY fires → traits = [Reliable / Curious / Helpful] (this overlap with the Peta/Nori briefs is documented — Nori coverage exists in `fix-worker-personality-coverage-extend-to-nori`, but the CROSS-SURFACE single-source consolidation is the underlying defect)
- Mariah Skills: `sample-worker-skills.ts` authored against a B2B-procurement persona that doesn't match the marketplace cohort badge

This is the "parallel abstractions" pattern from `docs/skycot/DRIFT_PREVENTION.md`: 5 places of truth, each diverging.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F7 [P1] — Persona drift between `/team` card and `/workers/<id>` Overview (cohort, traits, level)
- scene-1-and-2-claim-1-and-2 F11 [P2] — Mariah Skills tab lists B2B-procurement skills not matching marketplace cohort

## Fix recipe

Three coupled changes; ship as one PR.

### Phase 1: Pick the canonical source

`web/lib/sample-personas.ts` is meant to be the spine (per its header). Make it authoritative:
1. Move ALL persona-shape data (cohort, level, traits, greatAt, escalateWhen, signatureBehaviors, role, description) into `sample-personas.ts` as the single source.
2. Re-export getters: `findSamplePersona(workerId)`, `findSamplePersonaByName(name)`.

### Phase 2: Make derived files thin

Update each downstream consumer to READ from `sample-personas.ts`:

1. `web/lib/sample-team.ts` — derives cohort badge + description from `findSamplePersona(id).cohort` + `.description`
2. `web/components/skycot/TeammateCard.tsx` — `TRAITS_BY_NAME` + `LEVEL_BY_NAME` either deleted (read from sample-personas at render time) OR kept as a memoized derivation
3. `web/components/skycot/worker/WorkerPersonalityCard.tsx` — `PERSONALITIES` either deleted (read from sample-personas at render time) OR kept as a memoized derivation
4. `web/lib/sample-worker-skills.ts` — re-author Mariah's Skills tab to match her CANONICAL cohort (whichever one wins in Phase 1). If marketplace wins: drop the rfp-response/renewal-checklist/contract-redline triad, replace with marketplace skills (e.g. `listing.validate v2.0`, `pricing.score v1.4`, `photo.ocr v1.1`).

### Phase 3: Resolve the Mariah cohort question (operator-decisional)

Currently a real disagreement in the codebase. Two paths:

- **A** (keep `marketplace`): Mariah is the e-commerce/listings teammate. Update the detail-page hero to say "Marketplace · Listing operations" or similar. Re-author Skills tab to marketplace skills.
- **B** (keep `operations`): Mariah is the procurement teammate. Update the `/team` card cohort badge from MARKETPLACE to OPERATIONS. Skills tab stays as-authored.

The audit's audit-scene + the demo runbook's Mariah narration are agnostic between these. Pick one; document in the persona file header.

### Phase 4: Add a cross-surface vitest

`web/__tests__/persona-fixture-consistency.test.tsx`:
```ts
for (const persona of CANONICAL_PERSONAS) {
  const fromTeam = findInSampleTeam(persona.id);
  const fromCard = findInPersonalityCard(persona.name);
  const fromSkills = findInSampleWorkerSkills(persona.id);
  expect(fromTeam.cohort).toBe(persona.cohort);
  expect(fromCard.traits).toEqual(persona.traits);
  expect(fromSkills.activeSkills.map(s => s.id)).toMatchSnapshot();
}
```
Snapshot-tests pin the cross-surface answer once Phase 3 lands.

## Acceptance criteria

- [ ] Mariah's cohort badge on `/team` matches the cohort label on `/workers/<mariah-id>/overview` hero
- [ ] Mariah's Skills tab lists skills that semantically match her cohort
- [ ] Nori's traits on `/team` card === traits on `/workers/<nori-id>/overview` "HOW NORI WORKS"
- [ ] All 5 canonical personas (Mariah/Peta/Priya/Jamal/Nori) pass the cross-surface vitest
- [ ] `sample-personas.ts` is the single source of truth (other fixture files derive from it)
- [ ] `pnpm --dir web exec vitest run` green
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/lib/sample-personas.ts` (~60 LOC additions — consolidate cohort/role/skills metadata into the spine)
- `web/lib/sample-team.ts` (~30 LOC — switch to derived reads)
- `web/components/skycot/TeammateCard.tsx` (~20 LOC — switch to derived reads or delete TRAITS_BY_NAME/LEVEL_BY_NAME)
- `web/components/skycot/worker/WorkerPersonalityCard.tsx` (~20 LOC — switch to derived reads)
- `web/lib/sample-worker-skills.ts` (~40 LOC — re-author Mariah's skills, others stay)
- `web/__tests__/persona-fixture-consistency.test.tsx` (~80 LOC new spec)

## Out of scope

- Real-data persona shapes (this is fixture-spine; once the gateway returns real worker overviews, the fixtures are bypassed)
- The Peta-personality-coverage brief (already in flight; this brief extends to Nori + Mariah cohort/skills + cross-surface consolidation)
- C2 personality VISUAL design (locked; no rendering changes here)

## Effort

M. ~170 LOC + ~80 LOC test. ~2-3 hours. P1 because Scene 1 of the runbook narrates Mariah by cohort AND drills into her detail page — the camera sees the mismatch.
