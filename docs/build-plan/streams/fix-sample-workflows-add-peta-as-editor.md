# fix-sample-workflows-add-peta-as-editor

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-sample-workflows-add-peta-as-editor`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/scene-4-and-5-claim-4-and-5.md` F8 [P1]).

🟡 **P1 — The `lastEditor` field across the 7 sample workflows in `web/lib/sample-workflows.ts` distributes as Mariah ×2 / Priya ×1 / Jamal ×3 / Nori ×1. Peta is NEVER an editor. Per memory `project_pm_cohorts_peta_patrick` + the v1 demo runbook, the canonical demo 5 are Mariah / Peta / Priya / Jamal / Nori — Peta is one of them. Workflow cards on `/workflows` are visible behind the camera at multiple points in the demo, and the workforce shown there is silently missing the Project & Ops worker, contradicting the cohort canon.**

## Root cause

`web/lib/sample-workflows.ts:64-138` (SAMPLE_WORKFLOW_SUMMARIES) — 7 workflow entries with `lastEditor` field set to one of Mariah / Priya / Jamal / Nori. No entry uses Peta. `grep -n 'lastEditor' web/lib/sample-workflows.ts` returns 7 lines, Peta absent.

This is the same shape as `fix-workflow-card-roster-violations-rio-and-canonical-personas` (which fixed Rio → canonical 5) — that brief replaced wrong names; this brief covers the missing-Peta gap inside the right name set.

Note: cohort memory `project_pm_cohorts_peta_patrick` says Peta is the Project Coordinator L2 worker (Linear + GitHub stack). A Project & Ops workflow (e.g. `weekly_standup_digest` / `action_item_chase` / `sprint_demo_prep`) is the natural fit for her authorship.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/scene-4-and-5-claim-4-and-5.md` F8 [P1] — Workflow card "by Mariah / Priya / Jamal / Nori" never includes Peta — the canonical demo 5 are off-by-one

## Fix recipe

1. Add at least one Peta-authored workflow to `web/lib/sample-workflows.ts:64-138` SAMPLE_WORKFLOW_SUMMARIES. Suggested entry:

   ```ts
   {
     id: "weekly-standup-digest",
     name: "weekly_standup_digest",
     verticalId: "project-ops",
     summary: "Pulls last week's Linear + GitHub activity per teammate, drafts a 5-bullet standup digest, posts to Slack #standup channel.",
     channel: "Slack",
     lastEditor: "Peta",
     lastEditedAt: new Date(Date.now() - 86_400_000 * 2).toISOString(),
     runsLast7d: 5,
     // ... whatever other fields the WorkflowSummary type requires
   },
   ```

2. Add the matching entry to SAMPLE_WORKFLOW_SPECS at `web/lib/sample-workflows.ts:140-473` so drill-in renders correctly.

3. Optionally redistribute existing editors to give Peta a second authorship (e.g. swap `clean_customer_list` from Nori to Peta) so the distribution is Mariah ×2 / Peta ×2 / Priya ×1 / Jamal ×2 / Nori ×1 — more balanced.

4. Cross-check `crates/argo-demo-seed/src/workflow.rs` to keep the gateway-side seed in sync (operator may want the real backend to ship the Peta workflow too).

5. Audit cohort memory: confirm Peta is associated with `project-ops` vertical. If a different vertical-id is canonical, use that.

## Acceptance criteria

- [ ] At least one entry in `SAMPLE_WORKFLOW_SUMMARIES` has `lastEditor: "Peta"`.
- [ ] The matching `WorkflowSpec` exists in `SAMPLE_WORKFLOW_SPECS` so drill-in doesn't 404.
- [ ] `/workflows` page renders the new card behind the camera; in a screenshot the 5-roster canon is visually consistent.
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec vitest run lib/sample-workflows` passes; add a regression that asserts all 5 canonical personas (Mariah / Peta / Priya / Jamal / Nori) appear as `lastEditor` somewhere in the sample list.
- [ ] (Optional) `crates/argo-demo-seed/src/workflow.rs` updated to seed the matching real workflow under the demo tenant.

## May-touch

- `web/lib/sample-workflows.ts` (~30 LOC — 1 new summary + 1 new spec)
- `web/lib/__tests__/sample-workflows.test.ts` (~30 LOC new spec)
- `crates/argo-demo-seed/src/workflow.rs` (~30 LOC if extending the real seed too)

## Out of scope

- Patrick (the second Project & Ops worker) — covered separately if needed; this brief is only about Peta presence
- Rio / Maya / Sam non-canonical names elsewhere (covered by `fix-workflow-card-roster-violations-rio-and-canonical-personas` + `fix-persona-fixture-drift-mariah-cohort-traits-skills-cross-surface`)
- Peta personality / trait coverage on detail page (covered by `fix-peta-personality-trait-level-lookup-coverage`)

## Effort

S. ~60 LOC + test. ~45 min.
