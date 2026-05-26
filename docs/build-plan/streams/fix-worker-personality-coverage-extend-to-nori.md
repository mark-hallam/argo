# fix-worker-personality-coverage-extend-to-nori

**Owner**: claude.
**Authored against**: origin/main `74077bf676` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-personality-coverage-extend-to-nori`.
**Source**: ux-audit-2026-05-26-2214 `scene-1-and-2-claim-1-and-2.md` F5 [P1].

🟠 **P1 — The Peta personality-coverage brief (`fix-peta-personality-trait-level-lookup-coverage`) closed Peta-only, but this audit confirms Nori is ALSO missing from `PERSONALITIES` in `WorkerPersonalityCard.tsx:35-96`. Both Peta and Nori detail pages render as "Reliable / Curious / Helpful" generic boilerplate. Two of the five canonical workers read as placeholders on their own profile page. Same sibling-table sprawl bug.**

## Root cause

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` keys the `PERSONALITIES` map by worker name and only contains entries for Mariah / Priya / Jamal. The Peta brief is in flight; this brief extends the same fix to Nori AND adds residual cleanup that the Peta brief left out.

Three coupled defects to land in one PR:

1. **Nori missing from PERSONALITIES**:
   - `/workers/018f67d0-0000-7000-8000-000000000004/overview` body says "Reliable / Curious / Helpful" instead of Nori's actual traits ("Creative / On-brand / Concise / Curious" per her TeammateCard chips)
   - `rg 'Nori' web/components/skycot/worker/WorkerPersonalityCard.tsx` → zero matches

2. **Nori missing from `TeammateCard.tsx TRAITS_BY_NAME` + `LEVEL_BY_NAME`** (paired with the Peta fix — verify scope):
   - The audit confirmed Nori's `/team` deck card shows correct chips, so she IS in `TRAITS_BY_NAME` (line 51-67) but not in `PERSONALITIES`. The Peta brief covered both tables; this brief only needs to add to `PERSONALITIES` for Nori.

3. **Dead residual entries `Sam` + `Rio` in `TRAITS_BY_NAME` + `LEVEL_BY_NAME`** (`TeammateCard.tsx:56-57, 65-66`):
   - Leftover from the #2367 canonical-roster scrub. Should be deleted in this same PR.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F5 [P1] — `WorkerPersonalityCard` PERSONALITIES missing BOTH Peta AND Nori (Peta covered separately; this brief covers Nori + Sam/Rio cleanup)

## Fix recipe

1. **Pre-flight**: confirm the Peta brief (`fix-peta-personality-trait-level-lookup-coverage`) has merged. If still in flight, coordinate to land them in one PR or land Peta first.

2. **Add Nori to `PERSONALITIES`** at `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96`. Pull her content from `web/lib/sample-personas.ts findSamplePersona("018f67d0-0000-7000-8000-000000000004")` to keep one source of truth.

   Approximate shape (verify against `sample-personas.ts`):

   ```ts
   Nori: {
     traits: ["Creative", "On-brand", "Concise", "Curious"],
     greatAt: [
       "Drafting first passes of marketing copy from a brief",
       "Keeping voice + tone consistent across channels",
       "Asking before publishing anything customer-facing",
     ],
     escalateWhen: [
       "Brand-tone calls live outside the style guide",
       "A campaign needs final approval from leadership",
     ],
     signatureBehaviors: [
       "Drops two alternative angles before picking a winner",
       "Tags drafts with the voice spec it was written against",
     ],
   },
   ```

3. **Delete Sam + Rio entries** from `TeammateCard.tsx:56-57` (TRAITS_BY_NAME) + `TeammateCard.tsx:65-66` (LEVEL_BY_NAME). Verify no other callsite depends on them — `rg '"Sam"|"Rio"' web/` should only hit the lines being removed.

4. **Architectural follow-up (NOT in this brief)**: refactor `WorkerPersonalityCard`, `TeammateCard.TRAITS_BY_NAME`, and `LEVEL_BY_NAME` to all consume `findSamplePersona(workerId).traits` etc. from `web/lib/sample-personas.ts` (single source of truth). Author as `fix-worker-personality-tables-derive-from-sample-personas` if it doesn't already exist (see existing `fix-worker-personality-and-level-derive-from-real-data`).

5. **Update vitest spec** at `web/components/skycot/worker/__tests__/WorkerPersonalityCard.test.tsx` (or sibling location):
   - Assert Nori renders her actual traits, not "Reliable / Curious / Helpful"
   - Assert Sam / Rio are not present in `TRAITS_BY_NAME` (grep test)

## Acceptance criteria

- [ ] `/workers/018f67d0-0000-7000-8000-000000000004/overview` (Nori) renders her actual traits + greatAt + escalateWhen + signatureBehaviors — not the DEFAULT_PERSONALITY boilerplate
- [ ] `Sam` + `Rio` removed from `TeammateCard.tsx TRAITS_BY_NAME` + `LEVEL_BY_NAME`
- [ ] vitest spec covers Nori rendering + Sam/Rio absence
- [ ] `pnpm --dir web exec vitest run components/skycot/worker` green
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/components/skycot/worker/WorkerPersonalityCard.tsx` (~25 LOC Nori entry)
- `web/components/skycot/TeammateCard.tsx` (~4 LOC removal of Sam/Rio)
- `web/components/skycot/worker/__tests__/WorkerPersonalityCard.test.tsx` (~30 LOC test additions)

## Out of scope

- Refactor to derive personality from `sample-personas.ts` (covered by `fix-worker-personality-and-level-derive-from-real-data`)
- Peta-only fix (covered by `fix-peta-personality-trait-level-lookup-coverage`)
- The locked C2 design (no rendering changes — only fixture coverage)

## Effort

~30 LOC change + ~30 LOC test. ~30 min. P1 because Scene 1 drill-in to Nori from the 5-card deck reads as a placeholder on a canonical-roster worker, contradicting Claim 1.
