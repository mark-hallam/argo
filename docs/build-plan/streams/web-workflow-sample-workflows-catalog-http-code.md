# web-workflow-sample-workflows-catalog-http-code

**Owner**: claude.
**Depends on**: `web-workflows-http-and-code-nodes` MERGED (✅ via #2011).
**Authored against**: origin/main `b1f23a73e` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-sample-workflows-catalog-http-code`.

🟢 **P3 SPA SAMPLE-DATA POLISH.** Source: #2011 PR-body deferral ("`WorkflowStep.httpRequest` / `WorkflowStep.code` config fields added to the canonical builder `types.ts` only — `web/lib/sample-workflows.ts`'s catalog skipped"). #2011 shipped HTTP + Code step kinds in the builder, but the sample-workflows catalog (used in the v1 onboarding wizard + the "Try a workflow" empty state) still only references the legacy step kinds. New users on a cold install never see HTTP / Code examples.

## Scope

Extend `web/lib/sample-workflows.ts` so the catalog includes at least one workflow per new step kind: one with an HTTP request step (e.g. "Fetch and summarise an API"), and one with a Code step (e.g. "Transform CSV with a few lines of JS"). Each sample is a complete `WorkflowDefinition` matching the canonical `types.ts` shape from #2011.

Names + descriptions must be short, plain-English, and immediately useful to a new SKYCOT user. The sample workflows are what they see when they hit "Try a workflow" — make them feel like first-class examples, not placeholders.

If the sample-workflows catalog has a known maximum count (e.g. 5–7 to fit a grid), keep the catalog at the same size by RETIRING the least-loaded sample. Don't blow out the catalog UI just to add two new ones.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/lib/sample-workflows.ts` — add HTTP-step + Code-step sample workflows; retire less-relevant ones if the catalog has a size budget.
- `web/lib/sample-workflows.test.ts` (or co-located) — extend the round-trip / shape-validation tests.
- `web/app/(setup)/wizard/sample-workflow-card.tsx` (or similar — find the consumer) — narrowly scoped: ONLY if a new sample-kind icon/category is needed; verify before editing.

## Files you must NOT touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts` — the canonical shape from #2011 is locked.
- The step encoder (`step-encoder.ts`) — round-trip is already tested.
- Any gateway code.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] `web/lib/sample-workflows.ts` includes at least one workflow using an HTTP-request step kind.
- [ ] `web/lib/sample-workflows.ts` includes at least one workflow using a Code step kind.
- [ ] Both sample workflows match the canonical `WorkflowDefinition` shape from `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts` — they pass the existing round-trip test.
- [ ] Sample names are plain English (no internal jargon).
- [ ] The catalog UI (whatever consumes sample-workflows) renders both new samples without errors.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm lint` clean.
- [ ] `pnpm test --run` passes on the test file.
- [ ] Manual smoke noted in PR body: open `/workflows/<v>/<n>` empty state, see new HTTP + Code sample cards.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-workflow-sample-workflows-catalog-http-code`
- PR title: `web(workflow): sample workflows catalog includes HTTP + Code steps (web-workflow-sample-workflows-catalog-http-code)`

## Notes for the implementer

- ~100-200 LOC expected. Most is the two new sample workflow definitions.
- Pick concrete, immediately-useful samples — not "do nothing" placeholders. Suggested HTTP sample: "Daily weather email" (HTTP get → email step). Suggested Code sample: "Clean up customer names" (CSV in → JS string-normalisation → CSV out). Adjust to match what step kinds actually exist in the catalog today.
- Round-trip the new samples through the step encoder. If the encoder rejects them, the canonical shape from #2011 is off — flag in the PR body, do NOT silently work around it.
- Don't add new step kinds. If a sample needs a step kind that doesn't exist yet, redesign the sample to use only the existing ones.
