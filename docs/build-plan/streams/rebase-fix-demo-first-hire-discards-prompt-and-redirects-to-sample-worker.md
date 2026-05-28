# rebase-fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker

Authored against: origin/main `0c7cff50f5` on 2026-05-29
Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker

## Why this brief exists

PR #2463 (344 LOC, 2026-05-27) targeted the bug: in demo mode the first-hire mutation discards the operator's typed prompt and redirects to a fixed sample worker keyed off `verticalId` (`VERTICAL_TO_SAMPLE_WORKER` maps marketing → Nori, recruiter → Priya, etc). The /workers + /team roster never grows; the camera-visible defect is a Maya prompt landing on a Nori card — function-side fabrication called out by `feedback_no_theater_in_demo`.

The original PR's approach — `buildDemoFirstHireResult` parses worker name from description, mints UUID v7, appends to demo roster via `appendDemoWorker`, adds dismissable `DemoHireBanner` on `/workers/<id>/overview?demo-hire=1`, preserves query params on /workers → /team redirect, routes first-hire to `?demo-hire=1&just-hired=1&new=<id>` — is STILL load-bearing.

But the file shape has evolved significantly:
- **#2619** (parameterise first-hire example prompt by vertical) added `web/lib/onboarding/example-prompts.ts` and changed how the placeholder resolves.
- **#2493** softened first-hire copy on overview page.
- **#2493 + #2466 + #2548 + #2484** all touched the first-hire wizard text.
- **#2477** persists first-hire prompt and provider key across step-back (uses Zustand draft store) — needs to coexist with description-driven name parsing.
- **#2455 (still OPEN)** proposes a separate `name` input field instead of parsing from description.

Also load-bearing: the `feedback_scene_1_canonical_picker_was_wrong_for_mvp_genesis` memory + the **MVP-with-Genesis** direction mean the first-hire flow should ultimately be Genesis-driven (SSE `/api/v1/genesis` stream consumer). The description-driven roster-growth approach this brief proposes IS compatible with the Genesis MVP direction (the description IS the Genesis seed), but needs to coordinate with the Genesis dispatch in `crates/argo-cli/src/handlers/gateway/routes_onboarding_genesis.rs`.

PR #2463 was closed for re-author. Re-implementation should:

1. Check whether #2455 (explicit name field) lands first — if yes, source `name` from the explicit field; if no, port the description-parsing approach from #2463.
2. Either way, **stop the VERTICAL_TO_SAMPLE_WORKER short-circuit** at `web/lib/onboarding/use-first-hire.ts:121-144`.
3. Stage a real session-local hire that grows the demo roster.

## Source of truth

- Original brief: `docs/build-plan/streams/fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker.md`
- Closed PR: #2463
- Current main shape:
  - `web/lib/onboarding/use-first-hire.ts:121-144` (`VERTICAL_TO_SAMPLE_WORKER` short-circuit still active).
  - `web/app/(setup)/onboarding/first-hire/page.tsx` evolved through #2619 (example prompts by vertical).
  - `web/app/(app)/workers/[workerId]/overview/page.tsx` evolved through #2493 (softened copy).
- Sibling open PR: #2455 (explicit name field) — decide ordering before re-claim.

## Acceptance

Same as original brief plus:
- Coordinate with #2619 example-prompt lookup (don't regress placeholder behaviour).
- Coordinate with #2455 if/when it lands (use the explicit name field if present).
- Coordinate with `feedback_scene_1_canonical_picker_was_wrong_for_mvp_genesis` — the Genesis-driven flow is the MVP target; description-driven roster-growth is the bridge that lets the demo MP4 record before Genesis end-to-end is verified.

## Boundary

- Same files as original brief: `web/lib/onboarding/use-first-hire.ts`, `web/app/(app)/workers/page.tsx`, `web/app/(setup)/onboarding/first-hire/page.tsx`, `web/app/(app)/workers/[workerId]/overview/page.tsx`, the test files.

## Verification

- `npx vitest run lib/onboarding/__tests__/use-first-hire.test.ts tests/onboarding-demo-mode-bypass.test.ts`
- `npx vitest run __tests__/workers/empty-state-matches-team.test.tsx`
- `npx tsc --noEmit`
- `npx next lint --dir lib/onboarding --dir 'app/(app)/workers' --dir 'app/(setup)/onboarding/first-hire'`
