# mission-demo-seed-worker-roster-canon-peta-included

**Owner**: claude or codex.
**Authored against**: origin/main `bec35dd45` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-worker-roster-canon-peta-included`.
**Depends on**: `mission-demo-seed-data-bootstrap` (BLOCKED) — this brief tightens the data the master bootstrap will produce.

🟡 **P1 DEMO-GATE — seed worker roster must match runbook canon.**

The existing `mission-demo-seed-data-bootstrap` brief says: "6 named workers (Mariah, Priya, Jamal, Nori + 2 more)". The runbook canon roster (per `v1-mission-mvp-runbook.md` Scene 1 + memory `project_pm_cohorts_peta_patrick`) is:

| Slot | Name | Role | Notes |
|---|---|---|---|
| 1 | **Mariah** | Marketplace seller | Scene 1 narration anchor (referenced in audit drill-in) |
| 2 | **Peta** | Project Coordinator L2 | First non-vertical PM cohort worker per memory — P-names convention |
| 3 | **Priya** | Customer Success | Scene 5 narration anchor (zendesk + macos_notes combo) |
| 4 | **Jamal** | Sales SDR | Scene 4 narration support |
| 5 | **Nori** | (TBD — runbook role TBD) | Scene 1 fifth card |

So the canon is **5 workers** Mariah/Peta/Priya/Jamal/Nori, not 6. **Peta MUST be in the seed roster** — without her, the "first non-vertical PM cohort family" thread (memory `project_pm_cohorts_peta_patrick`) doesn't have its visible anchor. The earlier "+2 more" wording in the bootstrap brief is what created the canon drift.

## Scope

Edit `docs/build-plan/streams/mission-demo-seed-data-bootstrap.md` in place:

1. Change the worker count from 6 → 5 (or keep 6 and add a 6th P-name PM worker like Patrick per memory — operator should clarify).
2. List the canon roster explicitly: Mariah / Peta / Priya / Jamal / Nori.
3. Reference memory `project_pm_cohorts_peta_patrick` for Peta's persona definition.
4. Add Peta's persona to `web/lib/sample-personas.ts` IF NOT already present (verify with `grep -i 'peta' web/lib/sample-personas.ts`; the seed CLI reads personas from there per the master brief's `Files you may touch` list).
5. Update any pre-flight assertion that says `count == 6` to `count == 5` (or to whichever number the operator decides).

## Open question for operator

Choose ONE — the existing bootstrap brief said "6 workers". The runbook canon shows 5 (Mariah/Peta/Priya/Jamal/Nori). Options:

- **Option A**: 5 workers, exactly canon. Simpler narration; no surplus on /workers.
- **Option B**: 6 workers — canon 5 + Patrick (the P-name PM Status Reporter L3 from memory `project_pm_cohorts_peta_patrick`). Showcases the "Project & Ops cohorts" tab on /cohorts during demo.

The brief should be claimed with the operator's decision noted in the PR body.

## Acceptance criteria

- [ ] `mission-demo-seed-data-bootstrap.md` roster section explicitly lists Mariah / Peta / Priya / Jamal / Nori (+ Patrick if Option B).
- [ ] Peta's persona is in `web/lib/sample-personas.ts` with role, headline, vertical_id, traits.
- [ ] Any `count == 6` assertions in the bootstrap brief updated to the chosen number.
- [ ] PR body documents the Option A vs B decision.

## May-touch

- `docs/build-plan/streams/mission-demo-seed-data-bootstrap.md`
- `web/lib/sample-personas.ts` (add Peta + optionally Patrick)
- This brief file (status updates)

## Out of scope

- Implementing the seed CLI (separate brief)
- Touching crates/argo-demo-seed (separate brief)

## Effort

~15-30 minutes — doc + persona edit only.
