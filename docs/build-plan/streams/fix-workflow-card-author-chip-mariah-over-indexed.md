# fix-workflow-card-author-chip-mariah-over-indexed

**Owner**: claude or codex.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-card-author-chip-mariah-over-indexed`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/scene-4-and-5-claim-4-and-5.md` F10 [P2].

🟡 **P2 — All 8 seed workflows' author chips distribute Mariah×3, Priya×1, Jamal×3, Nori×1, Peta×1. Mariah appears on 3 of 8 cards (post_listing / morning_pipeline_digest / qualify_lead) — she is also the canonical Scene 1+2 persona, so the catalog screen the camera sweeps during Scene 5 lands an over-indexed roster signal. The author chip is dressing-side (fictional fixtures), but the camera sweeps the catalog and the eye lands on the chip distribution.**

## Root cause

`web/lib/sample-workflows.ts:65-149` declares 8 `WorkflowSummary` records. The `lastEditor` field carries the persona name. Current distribution:

- Mariah: 3 (post_listing, morning_pipeline_digest, qualify_lead)
- Jamal: 3 (varies by seed order)
- Priya: 1
- Nori: 1
- Peta: 1
- Patrick (the 6th persona per memory `project_pm_cohorts_peta_patrick`): 0

The 6-persona roster wants ~roughly even distribution across 8 cards: each persona authors 1-2 workflows. Mariah at 3 is over-indexed; Patrick at 0 misses an introduction opportunity.

This is dressing-side and unlikely to break Scene 5, but visible during the camera sweep. The fix is a 1-file rebalance of the `lastEditor` field.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F10 [P2] — Workflow card author chip credits over-index Mariah (3 of 8 cards)

## Fix recipe

1. **Edit `web/lib/sample-workflows.ts:64-149` `lastEditor` fields** to a more even spread:
   ```
   post_listing            → Patrick (was Mariah)  — introduces Patrick the Status Reporter on the catalog
   morning_pipeline_digest → Priya   (was Mariah)
   qualify_lead            → Mariah
   approve_refund          → Jamal
   screen_candidate        → Nori
   triage_ticket           → Jamal
   clean_customer_list     → Peta
   weekly_standup_digest   → Patrick
   ```
   Final distribution: Mariah×1, Priya×1, Jamal×2, Nori×1, Peta×1, Patrick×2.
2. **Verify Patrick is in the persona spine** (`web/lib/sample-personas.ts` — added per memory `project_pm_cohorts_peta_patrick`). If not yet, the audit reveals a missing-personas issue and this brief blocks until that lands. (Per the memory, Patrick is the L3 Status Reporter; verify the persona record exists.)
3. **Vitest** at `web/lib/sample-workflows.test.ts` (or new): assert each canonical persona name (Mariah/Priya/Jamal/Nori/Peta/Patrick) appears as `lastEditor` ≥1 time; assert no single persona appears >2 times.
4. **Update audit dressing-side acceptance** in the runbook: the camera sweep should land on at least one Patrick card so the workflow catalog matches the cohort page roster.

## Acceptance criteria

- [ ] No single persona is `lastEditor` on more than 2 of 8 seed workflows.
- [ ] Patrick appears as `lastEditor` on ≥1 seed workflow (introduces him to the catalog camera).
- [ ] All 6 canonical personas (Mariah/Priya/Jamal/Nori/Peta/Patrick) each appear at least once.
- [ ] Vitest covers the distribution invariants.
- [ ] Existing workflow-detail / build / runs tests still pass (lastEditor rename is data-only).

## May-touch

- `web/lib/sample-workflows.ts:65-149` (the `lastEditor` fields on the 8 seed `WorkflowSummary` records)
- `web/lib/sample-workflows.test.ts` (vitest invariant — NEW or update)
- `web/lib/sample-personas.ts` (verify Patrick record exists)

## Out of scope

- Author-type discriminant chip (covered by `fix-workflow-card-author-type-discriminant-chip`).
- Persona record creation for Patrick if missing (would be its own brief — covered by `fix-sample-workflows-add-peta-as-editor` precedent, or by the persona-spine extension memory).
- Substrate chip rebalance (covered by `fix-workflow-substrate-chip-show-macos-browser-e2b`).

## Effort

~20 LOC across the seed file + ~30 LOC vitest. ~20 min. P2 because the camera sweep is brief and the chip is one of many visual elements; not load-bearing on any narration beat.
