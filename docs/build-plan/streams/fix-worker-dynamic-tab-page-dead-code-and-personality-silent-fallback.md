# fix-worker-dynamic-tab-page-dead-code-and-personality-silent-fallback

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-dynamic-tab-page-dead-code-and-personality-silent-fallback`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-1-and-2-claim-1-and-2.md` F15 [P2] + F16 [P2].

🟡 **P2 — Dead-code drift on the worker-detail route: `web/app/(app)/workers/[workerId]/[tab]/page.tsx` is the dynamic-segment stub that renders "Overview — coming in E4-b" etc., but the seven sibling static routes (overview/activity/memory/skills/trust/escalation/settings) all exist and are real. Next.js prefers static segments, so the dynamic stub is unreachable — pure vestigial code referencing long-finished briefs. Separately, `web/components/skycot/worker/WorkerPersonalityCard.tsx:117` silently falls through to `DEFAULT_PERSONALITY` when `PERSONALITIES[worker.name]` misses, without any `console.warn` — exactly the pattern that let the Peta + Nori gap slip into production for weeks.**

## Root cause

### F16 — dead-code stub

`web/app/(app)/workers/[workerId]/[tab]/page.tsx` exists and renders `TAB_BRIEFS[tab]` placeholders ("Overview — coming in E4-b"). The seven sibling static segments exist:
- `web/app/(app)/workers/[workerId]/overview/page.tsx`
- `web/app/(app)/workers/[workerId]/activity/page.tsx`
- `web/app/(app)/workers/[workerId]/memory/page.tsx`
- `web/app/(app)/workers/[workerId]/skills/page.tsx`
- `web/app/(app)/workers/[workerId]/trust/page.tsx`
- `web/app/(app)/workers/[workerId]/escalation/page.tsx`
- `web/app/(app)/workers/[workerId]/settings/page.tsx`

Next.js routing always picks the static segment first, so the `[tab]/page.tsx` is unreachable. The `TAB_BRIEFS` map references brief ids (`E4-b`, etc.) that landed long ago. Dead-code-drift per the CLAUDE.md rule.

### F15 — silent default-personality fallback

`web/components/skycot/worker/WorkerPersonalityCard.tsx:117` does:
```ts
const personality = PERSONALITIES[worker.name] ?? DEFAULT_PERSONALITY;
```

No `console.warn` when the map misses. The Peta + Nori production gap (covered by `fix-peta-personality-trait-level-lookup-coverage`) slipped because the silent fallback rendered believable-but-generic content. A dev-mode `console.warn("Missing personality for", worker.name)` would have flagged it on first render in CI / Playwright runs.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F15 [P2] — Worker personality card falls through to DEFAULT_PERSONALITY silently — no console warning
- scene-1-and-2-claim-1-and-2 F16 [P2] — `/workers/{id}/[tab]/page.tsx` exists but is dead code — never reachable

## Fix recipe

### F16 dead-code delete

1. **Delete** `web/app/(app)/workers/[workerId]/[tab]/page.tsx`.
2. **Delete** `web/app/(app)/workers/[workerId]/[tab]/layout.tsx` (verify it exists; also vestigial).
3. **Verify** no imports reference these files. Run `grep -rn "TAB_BRIEFS\|\\[tab\\]" web/` and clean up any orphans.
4. **Cross-check** with the routing — visit `/workers/{id}/overview`, `/workers/{id}/skills` etc. and confirm they still render correctly.

### F15 dev-mode warn

5. **Add a dev-only warn** at `web/components/skycot/worker/WorkerPersonalityCard.tsx:117`:
   ```ts
   const personality = PERSONALITIES[worker.name];
   if (!personality && process.env.NODE_ENV !== "production") {
     console.warn(`[WorkerPersonalityCard] Missing personality for "${worker.name}". Add to PERSONALITIES map.`);
   }
   const resolved = personality ?? DEFAULT_PERSONALITY;
   ```
6. **Same pattern at `TeammateCard.tsx`** for `TRAITS_BY_NAME` and `LEVEL_BY_NAME` lookups (`web/components/skycot/TeammateCard.tsx:51-67`):
   ```ts
   if (!TRAITS_BY_NAME[worker.name] && process.env.NODE_ENV !== "production") {
     console.warn(`[TeammateCard] Missing traits for "${worker.name}". Add to TRAITS_BY_NAME.`);
   }
   ```

### Tests

7. **Add a Playwright spec** `web/playwright/worker-detail/no-dead-code-and-missing-warn.spec.ts`:
   - Assert visiting `/workers/{id}/random-non-existent-tab` returns the standard Next.js 404 (not a TAB_BRIEFS stub).
   - Boot in dev mode with `console` event capture.
   - Visit `/workers/{id}/overview` for each of the 5 canonical personas (Mariah / Peta / Priya / Jamal / Nori).
   - Assert NO `[WorkerPersonalityCard] Missing personality` warning fires (i.e. all 5 have personality entries).

## Acceptance criteria

- [ ] `web/app/(app)/workers/[workerId]/[tab]/page.tsx` and sibling `layout.tsx` are deleted.
- [ ] All 7 tab routes still render correctly.
- [ ] Personality + traits + level lookups emit a dev-mode `console.warn` when the map misses.
- [ ] Production builds suppress the warns.
- [ ] New Playwright spec passes.

## May-touch

- `web/app/(app)/workers/[workerId]/[tab]/page.tsx` (DELETE)
- `web/app/(app)/workers/[workerId]/[tab]/layout.tsx` (DELETE — verify exists)
- `web/components/skycot/worker/WorkerPersonalityCard.tsx:117`
- `web/components/skycot/TeammateCard.tsx:51-67`
- `web/playwright/worker-detail/no-dead-code-and-missing-warn.spec.ts` (NEW)

## Out of scope

- Backfilling Peta / Nori personality data (covered by `fix-peta-personality-trait-level-lookup-coverage`).
- Refactoring the 3 parallel maps into `sample-personas.ts` (covered by Option B in the same Peta brief).
- Worker pronoun policy (covered by `fix-worker-pronouns-policy-they-or-explicit-pronoun-field`).

## Effort

~30-80 LOC (mostly deletes + warns). ~2-3 hours. P2 — code-hygiene fixes that prevent recurrence of the Peta-style production gap.
