# fix-playwright-misc-regression-cluster

**Owner**: codex.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-playwright-misc-regression-cluster`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` Playwright failure cluster: Approvals (3) + Team (3) + Audit (2) + Misc (8).

🟡 **P2 — A long tail of Playwright specs failing across approvals filter / inbox / live, team filter-search + page-shows-real-workers, audit cross-system-links + filter-popover, cohorts-project-ops, orchestration-queue, skill-canvas-editor, trust-promotion. Total 16 specs, each likely a small fix; consolidating into one cleanup brief.**

## Root cause

Multiple unrelated regressions that share the pattern of "small, focused spec failing because of an under-the-hood data shape change or DOM selector drift". Likely root causes:

1. **Approvals specs (3)** — `approvals-filter`, `approvals-inbox`, `approvals-live` — downstream of the env-var typo + fixture-drift (covered by parent briefs). May green automatically once those land.
2. **Team page specs (3)** — `team-filter-search`, `team-page-shows-real-workers` × 2 — downstream of the gateway-binary-stale + workers-collection-alias problem (covered by separate briefs).
3. **Audit specs (2)** — `audit-cross-system-links`, `audit-filter-popover` — likely small DOM/selector drift or downstream of audit-action-detail 500 fallback fix.
4. **Cohorts** (1) — `cohorts-project-ops` — possibly downstream of the Peta persona missing from lookup tables (covered separately).
5. **Orchestration** (1) — `orchestration-queue` — likely no workflow runs in seed; downstream of workflow-seed bootstrap.
6. **Skill canvas** (1) — `skill-canvas-editor` — possibly tied to a recent skill-detail refactor.
7. **Trust promotion** (1) — `trust-promotion` — possibly tied to permissions / freeze-button changes.

## Findings this brief closes

- chrome-live-walk-and-playwright Playwright failures (misc cluster, 16 specs)
  - `approvals-filter`, `approvals-inbox`, `approvals-live`
  - `team-filter-search`, `team-page-shows-real-workers` × 2
  - `audit-cross-system-links`, `audit-filter-popover`
  - `cohorts-project-ops`
  - `orchestration-queue`
  - `skill-canvas-editor`
  - `trust-promotion`

## Fix recipe

1. **Read the full playwright run.log** at `docs/skycot/ux-audits/2026-05-26-2007/playwright/run.log` — categorize each of the 16 failures by error mode (selector miss / timeout / assertion fail / server error).

2. **Wait for parent briefs to land first** (don't fix downstream symptoms before root causes):
   - `fix-onboarding-provision-demo-mode-bypass`
   - `fix-claude-launch-json-env-var-naming-mismatch`
   - `fix-approvals-inbox-waiting-on-you-fixture-drift`
   - `fix-skills-marketplace-perpetual-loading`
   - `fix-audit-action-detail-demo-mode-fallback`
   - `fix-peta-personality-trait-level-lookup-coverage`
   Re-run the 16 specs; document which ones greened up automatically and which still fail.

3. **For each remaining failure**, fix in-place. Most should be 1-5 LOC selector or fixture updates.

4. **For the 1-2 that turn out to be larger** (e.g. `skill-canvas-editor` may need real editor wiring), spin them off into dedicated briefs and mark them `test.skip` here with a TODO referencing the spawn-off brief.

## Acceptance criteria

- [ ] `pnpm playwright test` overall failure count drops by ≥14 (out of the 16 listed)
- [ ] Any spec that can't be made green in this brief is explicitly `.skip`-ed with a comment pointing at a follow-up brief
- [ ] No spec marked `.skip` without operator review

## May-touch

- `web/playwright/approvals/*.spec.ts`
- `web/playwright/team/*.spec.ts`
- `web/playwright/audit/*.spec.ts`
- `web/playwright/cohorts/*.spec.ts`
- `web/playwright/orchestration/*.spec.ts`
- `web/playwright/skills/*.spec.ts`
- `web/playwright/trust/*.spec.ts`
- Whatever component files the specs probe

## Out of scope

- The parent root-cause briefs (must land first)
- Net-new feature work — only regression cleanup
- Spec coverage expansion (separate concern)

## Effort

~50-300 LOC across many small fixes + investigations. Heavily dependent on how many self-heal after parent briefs. ~6-12 hours. P2 because each individual spec is small but together they're meaningful noise in the suite.
