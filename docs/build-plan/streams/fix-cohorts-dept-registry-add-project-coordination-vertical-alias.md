# fix-cohorts-dept-registry-add-project-coordination-vertical-alias

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-cohorts-dept-registry-add-project-coordination-vertical-alias`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F02)

🟠 **P1 — `/cohorts` quietly drops the Peta (Project Coordinator) cohort into a tab labelled "Other" because the gateway returns `vertical: "project-coordination"` while the SPA dept-registry only matches `["project", "projects", "pm", "project-ops"]`. The locked D3 magazine tab labelled "Project & Ops" never renders even though Peta IS the headline PM cohort.**

## Root cause

`web/components/skycot/cohorts/dept-registry.ts:28-32` declares the Project & Ops department's vertical match list:

```ts
{
  id: "project-ops",
  label: "Project & Ops",
  verticals: ["project", "projects", "pm", "project-ops"],
  // ...
}
```

The gateway `/api/v1/cohorts` response carries Peta's cohort with `vertical: "project-coordination"` (the dash-separated form). The registry's `verticals.includes(vertical)` check fails on the exact string and falls through to `OTHER_DEPT`. Reproduction:

```
fetch('/api/v1/cohorts').then(r=>r.json())
→ cohort with { vertical: "project-coordination", … }
```

Observed `/cohorts` page renders 5 dept tabs: Support, Sales, People & HR, Marketing, **Other** — with Peta inside "Other". The locked design (memory `project_cohorts_page_design_locked`) explicitly names Project & Ops as a 1st-class tab and the PM cohorts (Peta, Patrick) were authored to populate it (memory `project_pm_cohorts_peta_patrick`).

Sibling brief `fix-cohorts-memory-drift-project-ops-tab-position` (TODO) updates the memory file; this brief is the actual code fix on the registry. They are independent.

## Findings this brief closes

- whole-site F02 [P1] — Peta cohort mis-buckets into "Other" because dept-registry doesn't match `project-coordination`

## Fix recipe

1. **Add `project-coordination` to the Project & Ops verticals array** in `web/components/skycot/cohorts/dept-registry.ts:28-32`:
   ```ts
   verticals: ["project", "projects", "pm", "project-ops", "project-coordination"],
   ```
2. **Audit the other 8 departments** in the same file for similar dash-vs-no-dash inconsistencies. The gateway uses kebab-case vertical IDs (`marketplace-seller`, `customer-support`, `gtm-engineer`, `marketing-intern`, `sales-sdr`, `finance-ar`, `recruiting`) — cross-check each dept's `verticals` array against `curl http://127.0.0.1:7400/api/v1/cohorts | jq '.cohorts[].vertical'` and add aliases as needed.
3. **Add a vitest spec** at `web/components/skycot/cohorts/dept-registry.test.ts` (NEW or extend) that asserts every vertical returned by the demo seed maps to a non-`other` department:
   ```ts
   const seedVerticals = ["marketplace", "project-coordination", "recruiting", "support", "marketing"];
   for (const v of seedVerticals) {
     expect(deptForVertical(v).id).not.toBe("other");
   }
   ```
4. **OPTIONAL: align the gateway and the SPA on a canonical vertical ID set** — out of scope here; the SPA-side alias is the safer, smaller fix per memory `feedback_brief_authoring_preflight_rules` (additive-not-refactor). Track the alignment work as a separate codex brief if the operator wants it.

## Acceptance criteria

- [ ] `/cohorts` renders a "Project & Ops" tab with Peta's cohort inside it (NOT in "Other").
- [ ] No cohort with a recognised vertical falls through to `OTHER_DEPT` in the demo seed.
- [ ] vitest `dept-registry.test.ts` passes with the new alias.
- [ ] No regression on the other 5 currently-rendering tabs (Support, Sales, People & HR, Marketing).
- [ ] PR body cites whole-site F02 and notes the additive nature (no removal of existing aliases).

## May-touch

- `web/components/skycot/cohorts/dept-registry.ts` (~3 LOC additive)
- `web/components/skycot/cohorts/dept-registry.test.ts` (NEW or extend, ~40 LOC)

## Out of scope

- Gateway-side vertical-id canonicalisation (separate codex brief, post-MVP)
- Cohorts cold-paint render hang (covered by separate brief on whole-site F03)
- Seeding the pitch-agent + 4 other missing cohorts (covered by separate brief on whole-site F06)
- Memory-file vocabulary fix (covered by `fix-cohorts-memory-drift-project-ops-tab-position`)

## Effort

XS — ~3 LOC additive change + ~40 LOC test. ~30-45 min.
