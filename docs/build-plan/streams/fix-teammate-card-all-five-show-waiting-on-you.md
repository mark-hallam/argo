# fix-teammate-card-all-five-show-waiting-on-you

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-teammate-card-all-five-show-waiting-on-you`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` F11 [P2]).

🟡 **P2 — All 5 teammate cards on `/team` are rendering with the same "Waiting on you" status pill, despite the `SAMPLE_TEAM` fixture giving them DIFFERENT statuses (Mariah/Priya/Peta = `waiting`; Jamal = `working`; Nori = `idle`). The visual rendering appears to collapse every status to the same pill. Reads as "5 teammates, one big to-do list" instead of "5 teammates, one team" — opposite of the hero copy.**

## Root cause

The audit's accessibility snapshot showed every card on the `/team` deck displaying the same "Waiting on you" status pill. The data layer at `web/lib/sample-team.ts` HAS varied statuses across the 5 workers (verified during audit). The likely culprits — pick one or both — are at `web/components/skycot/TeammateCard.tsx` in the status-pill render path:

1. **`STATUS_LABELS` map missing keys.** If the map only has an entry for `waiting` and falls back to "Waiting on you" for every other value, every card renders as `waiting`.
2. **The pill receives the wrong field.** If the card reads `worker.status` but the fixture uses `worker.state` (or vice-versa), every card gets `undefined` → fallback to "Waiting on you".
3. **The card is hard-coded to render `waiting` regardless of input** (CSS variant or prop default).

This needs a 10-minute read of `TeammateCard.tsx` to confirm which is the case. The fix is small but exact-needs-to-be-verified.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` F11 [P2] — All 5 trading cards show identical "Waiting on you" status

## Fix recipe

1. Read `web/components/skycot/TeammateCard.tsx`. Find the status-pill render block. Verify the data path: what field does the card consume? What does `STATUS_LABELS` map have?
2. Compare with `web/lib/sample-team.ts` field shape — does the fixture provide that field with varied values?
3. Pick the actual bug:
   - **If `STATUS_LABELS` is incomplete**: add entries for all status enum values (`waiting`, `working`, `idle`, plus any others — see `TeamMember` type definition).
   - **If field-name mismatch**: align the card to read whatever the fixture provides. If `WorkerSummary` type uses one name and `TeamMember` another, pick one and propagate.
   - **If hard-coded**: remove the hard-code; thread the prop through.
4. Test: in dev mode, open `/team`, eyeball the 5 cards — should see 3 distinct labels across them.

## Acceptance criteria

- [ ] On `/team` with `SAMPLE_TEAM` data, the 5 cards display at least 3 distinct status pills (not all "Waiting on you").
- [ ] Mariah / Priya / Peta render as `waiting` (label: "Waiting on you").
- [ ] Jamal renders as `working` (label: TBD per design — likely "Working on it" or "In progress").
- [ ] Nori renders as `idle` (label: TBD — likely "Idle" or "Available").
- [ ] `pnpm --dir web exec vitest run components/skycot/TeammateCard` passes with a new test that mounts 3 cards with different statuses and asserts 3 distinct labels.
- [ ] `pnpm --dir web exec tsc --noEmit` green.

## May-touch

- `web/components/skycot/TeammateCard.tsx` (~10-20 LOC — STATUS_LABELS map or field-read fix)
- `web/lib/sample-team.ts` (only if the fix flips the canonical field name)
- `web/components/skycot/__tests__/TeammateCard.test.tsx` (~40 LOC new spec)

## Out of scope

- The hero stat "0 working right now" (covered by `fix-team-page-hero-hardcoded-weekly-actions-hours-saved`)
- Adding new status enum values (e.g. `escalating`, `paused`) — separate brief if needed
- The MOCK-OFFLINE diagnostic chip (covered by `fix-team-page-mock-offline-chip-suppress-during-recording`)

## Effort

XS. ~30 LOC + test. ~30-45 min. P2 because the camera CAN pause on the deck and a viewer counts 5 identical pills + 1 hero stat "0 working" — implicit contradiction that erodes the "5 teammates, one team" narrative.
