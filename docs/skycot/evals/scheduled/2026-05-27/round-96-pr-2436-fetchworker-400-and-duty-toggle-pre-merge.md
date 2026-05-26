# Round 96 — PR #2436 T0519MEL P0s (fetchWorker 400 + duty toggle E4-h leak) PRE-MERGE

**Date**: 2026-05-26 (UTC) / 2026-05-27 (AEDT)
**Round**: 96
**Pillar / Claim**: Demo-MP4 P0 (UX-AUDIT-T0519MEL Brief 1 + Brief 3) — anti-theater + leakage
**Verdict**: 🟢 PASS — PRE-MERGE PASS (P0)

## Setup

PR #2436 head `d9455b091`, MERGEABLE per gh, +110/-53 across 5 files.
Operator-directed "do them yourself 2026-05-27"; bundles 2 of 3 T0519MEL P0
demo-blocker fixes (Brief 2 is a separate Rust gateway PR).

```bash
git fetch origin pull/2436/head:pr-2436 --quiet
git log -1 --format='%H %ai' pr-2436
# d9455b091b735a5c650edc8f1aeaab3937cd9d0f 2026-05-27 09:13:33 +1000
git diff origin/main pr-2436 -- web/lib/api/workers.ts web/lib/api/workers.test.ts \
  web/components/skycot/worker/DutiesCard.tsx \
  web/components/skycot/worker/WorkerOverviewContent.tsx \
  web/components/skycot/worker/Overview.test.tsx | head -300
pnpm vitest run lib/api/workers.test.ts components/skycot/worker/Overview.test.tsx
```

## Observed

**Brief 1 — `fix-fetchworker-400-bad-request-no-branch-infinite-retry`**
([web/lib/api/workers.ts:110-118](web/lib/api/workers.ts#L110-L118)):
the `isMissingOrUnreachable()` predicate and the explicit 4xx branch in
`fetchWorker()` both extended to match HTTP **400** (`invalid_worker_id`)
in addition to **404**. The demo wizard mints synthetic ids like
`demo-worker-marketing-intern-mpn1b10z` that fail the gateway's UUIDv7
parse and return 400. Before this branch: `ApiError` bubbled raw + React
Query retried indefinitely + WorkerLayoutShell stayed in `isPending` for
~15s before settling on the friendly empty state. After: 400 funnels into
`WorkerNotFoundError` on the same lane as 404.

**Brief 3 — `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak`**
([web/components/skycot/worker/DutiesCard.tsx:28-66](web/components/skycot/worker/DutiesCard.tsx#L28-L66) +
[web/components/skycot/worker/WorkerOverviewContent.tsx:60-83,141-145](web/components/skycot/worker/WorkerOverviewContent.tsx#L60-L83)):
this is the load-bearing change.

- New `canToggleDuty?: boolean` prop on `DutiesCard` defaulting to
  `canEdit` for back-compat. `WorkerOverviewContent` now passes
  `canToggleDuty={false}` explicitly while leaving `canEdit` true (so
  the +Add Responsibility dialog mount stays live).
- **Removed entirely** from `WorkerOverviewContent`: the `announcement`
  `useState` + the `useEffect` 3.2s timer + 3 toast strings (`"Add
  responsibility dialog ships in E4-h"`, `"Edit schedule ships in E4-h"`,
  `"Charter changes persist in E4-g"`) + 2 `console.warn` calls leaking
  `[E4-b]` brief-IDs + the `<div role="status" aria-live="polite"
  data-testid="overview-announcement">` ARIA-live region.
- The `DutyRow` body-click handler is now `canEdit ? () => onEditDuty?.(...)
  : undefined` + the button receives `disabled={!canEdit}` + the hover
  background is conditionally suppressed → the camera-visible body
  becomes a no-op disabled affordance, not a fake-save flash.

**Vitest re-run** (production code + new tests, real test runner, not PR-body claim):

```
pnpm vitest run lib/api/workers.test.ts components/skycot/worker/Overview.test.tsx
…
Test Files  2 passed (2)
     Tests  28 passed (28)
  Duration  81.76s
```

10 (workers.test.ts) + 18 (Overview.test.tsx) = 28 PASS, matches PR body
claim (`10/10 PASS` + `18/18 PASS`). The post-run `DOMException
[AbortError]` lines are happy-dom teardown noise (pre-existing flake
class — fetch async tasks pending at environment teardown); they do not
fail tests.

## Anti-theater check

Three independent axes confirm this isn't "tests rewritten to be green":

1. **The OLD assertion was `expect(getByTestId("overview-toast"))
   .toHaveTextContent(/E4-h/)`** — i.e. the prior test pinned that the
   leak EXISTED. The new test is **`expect(panel.textContent ?? "")
   .not.toMatch(/\bE\d+-[a-z]\b/)`** — a negative regex pin asserting NO
   engineering brief-ID anywhere in the panel text. If the toast UI was
   still rendering, this assertion would fail on regex match. The PR
   removes the entire announcement state + ARIA region from
   `WorkerOverviewContent`; the negative pin can only pass if the
   removal is real.

2. **Two new positive pins** ([Overview.test.tsx:259-269](web/components/skycot/worker/Overview.test.tsx#L259-L269))
   assert `duty-toggle-d1` and `duty-edit-d1` are both `.toBeDisabled()`.
   These read from real DOM elements rendered by `DutiesCard`, not from
   mocked spies. If `canToggleDuty={false}` had been a no-op prop, the
   buttons would remain enabled and the assertions would fail.

3. **workers.test.ts adds the `invalid_worker_id` 400 fixture body**
   ([workers.test.ts:127-135](web/lib/api/workers.test.ts#L127-L135))
   verbatim from the gateway error contract (`{ error: { code:
   "bad_request", message: "invalid worker_id '...'" } }`). The
   `fetchWorker` rejection is asserted via `.rejects.toBeInstanceOf(
   WorkerNotFoundError)` — class-instance check, not message string
   check, so the assertion can't be satisfied by a thrown `Error`
   that happens to be named "WorkerNotFoundError".

## Gap / follow-up brief candidates

- **🟢 P0 MERGE** PR #2436 under the permanent web-only fix-* lane (web
  files only, ≤5 files at +110/-53, vitest 28/28 GREEN, MERGEABLE per
  gh, anti-theater dispositive). Closes T0519MEL Brief 1 + Brief 3
  demo-blocker P0s.
- **🟢 P1 NEW (was carry — STILL UNAUTHORED)** — `fix-gateway-worker-duties-schedules-routes-404`
  and `fix-worker-detail-responsibilities-schedules-tab-end-to-end` are
  cited in the PR body's "Out of scope" section as the load-bearing
  follow-ups that flip `canToggleDuty={true}` back on. Recommend
  conductor cross-check that both briefs exist in `docs/build-plan/streams/`
  and are on the queue; if not, surface to operator.
- **🟡 P3 NEW** — `test-overview-leak-guard-extend-to-all-worker-tabs` (~30 LOC).
  The new negative regex pin `/\bE\d+-[a-z]\b/` is scoped to
  `worker-tab-panel-overview`; extending the same guard to the
  Schedules / Activity / Memory tabs would catch the next leak class
  before it reaches the camera.

## Process note

102nd anti-theater verification this week. The "remove the entire toast
UI + assert the removal via negative regex pin" pattern is the gold
standard for leak-class fixes — recommend it as a template in
CONTRIBUTING_AGENT.md or similar.
