# fix-workflow-card-roster-violations-rio-and-canonical-personas

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-card-roster-violations-rio-and-canonical-personas`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-4-and-5-claim-4-and-5.md` F10 [P1].

🟠 **P1 — Workflow card `qualify_lead` is attributed "by Rio" on the card meta row. Rio is not in the canonical demo roster (Mariah / Peta / Priya / Jamal / Nori per `feedback_no_theater_in_demo` + Scene 1 lockup). Camera reads vocabulary drift between Scene 1's roster and Scene 5's workflows.**

## Root cause

`web/lib/sample-workflows.ts:103` sets `lastEditor: "Rio"` on the `qualify_lead` template. The canonical roster is **Mariah / Peta / Priya / Jamal / Nori** per Scene 1. Sample-workflows.ts may have other roster violations — full sweep needed.

This is "scene-vocabulary mismatch" per the v1-demo positioning rule (`v1-demo-positioning-poc-vs-public.md`): the camera sees both Scene 1's "your team is Mariah/Peta/..." AND Scene 5's "edited by Rio" within minutes, and the inconsistency is camera-visible.

## Code evidence

`web/lib/sample-workflows.ts:103`:
```ts
lastEditor: "Rio",
```

Also `web/lib/sample-personas.ts:181-188` `SAMPLE_PERSONA_IDS` still includes `SAM / RIO` instead of `PETA`. Multiple files inherit the wrong roster.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F10 [P1] — Workflow card "by Rio" violates Scene-1 worker-roster canon

## Fix recipe

1. **Grep all sample fixtures for non-canonical persona names**:
   ```bash
   rg '"Sam"|"Rio"|"Maya"' web/lib/sample-*.ts web/lib/workflow-templates.ts
   ```
2. **Replace every Sam / Rio reference** with one of the canonical 5 (Mariah / Peta / Priya / Jamal / Nori). Preserve narrative variety — don't make every workflow "edited by Mariah". Suggested mapping:
   - Sales workflows (qualify_lead, draft_outreach, book_demo) → Mariah / Jamal
   - Marketing workflows (competitor_intel_digest, kb_freshness_check) → Maya rename to Mariah; or Peta
   - Customer-research / ops workflows → Peta / Priya
   - Support workflows (triage_ticket, refund_eligibility) → Priya
   - Finance workflows (dunning_followup) → Nori
3. **Update `SAMPLE_PERSONA_IDS`** in `web/lib/sample-personas.ts:181-188` to drop SAM + RIO and add PETA. Ensure the persona constant is defined (line ~122 per audit notes).
4. **Add a unit test** at `web/tests/sample-roster-canon.test.ts` that imports every sample fixture and asserts every `editor` / `author` / `creator` / `lastEditor` / `assignee` field is one of the canonical 5 names.
5. **Lint rule** (optional): add to `eslint-plugin-skycot` a check for hardcoded persona names outside the canonical list.

## Acceptance criteria

- [ ] `rg '"Sam"|"Rio"' web/lib/sample-*.ts web/lib/workflow-templates.ts` returns zero hits.
- [ ] `SAMPLE_PERSONA_IDS` contains exactly: MARIAH / PETA / PRIYA / JAMAL / NORI.
- [ ] Roster-canon test passes.
- [ ] Scene 5 camera-pan on `/workflows` shows only canonical names.
- [ ] No visual regression on `/workflows` card layout.

## May-touch

- `web/lib/sample-workflows.ts` (replace Rio etc.)
- `web/lib/sample-personas.ts:181-188` (SAMPLE_PERSONA_IDS)
- `web/lib/workflow-templates.ts` (verify no Rio refs)
- `web/lib/sample-audit.ts` (verify)
- `web/lib/sample-approvals.ts` (verify)
- `web/tests/sample-roster-canon.test.ts` (NEW)

## Out of scope

- Renaming the live gateway worker "Maya" → "Mariah" (gateway-side seed; covered by `fix-demo-seed-cli-for-real-roster`)
- Persona avatars (separate brief: `fix-persona-avatars-bundled-and-served`)
- Adding new personas beyond the canonical 5 (post-v1.0)

## Effort

~30-60 LOC + 1 test. ~1 hour. Pure mechanical mapping; high-confidence ship.
