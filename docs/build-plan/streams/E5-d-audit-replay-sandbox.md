# Stream E5-d — Audit timeline: replay sandbox (verdict + diff + provenance diff)

**Owner**: claude.
**Depends on**: E0 MERGED; E5-c MERGED.
**Authored against**: origin/main `634f1462` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E5-d-audit-replay-sandbox`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E5 brief 4 of 8.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Replay execution | `POST /api/v1/audit/<id>/replay` returns the replay result; client renders verdict + diff. Mock fallback returns a fixed `reproducible` verdict in dev. | Server-side sandbox enforced (no real side-effects — MCP calls go to a mock server, CU to virtual display, outbound channels produce would-have-sent traces) — already aligned with the architectural ground truth. | n/a |
| Side-effect quarantine | Server-side; client never executes anything | Same. Mandatory invariant. | n/a |
| Verdict trichotomy | Three states: `reproducible` / `drifted-aligned` / `drifted-misaligned` | Same. Operator-feedback brief may add more granular states later. | `mission-enterprise-replay-verdict-granularity` |
| Decommissioned-model fallback | Renders "Replay unavailable — Claude 4.7 decommissioned. Original output preserved below." | Same. v1 ships this graceful path. | n/a |
| Side-effect-leak detection | Server-side red-banner abort if sandbox attempts a real call | Same. Self-audit row written. | `mission-enterprise-replay-leak-detection` (already enforced server-side via existing trajectory-replay brief; client just renders) |
| Replay cost attribution | Free in v1 (replays run on a separate quota) | Per-tenant cost cap on replays; soft-warn at quota | `mission-enterprise-replay-cost-cap` |

v1 is acceptable: the replay sandbox already has its
backend brief (`claude-audit-replay` referenced in the
audit-timeline wireframe). This brief adds the UI on
top.

## Why

E5-c's "Replay" button routes to `/audit/action/<id>/
replay`. This brief lights up that surface: kicks off
the replay, renders verdict + output diff + provenance
diff per the wireframe.

For enterprise: replay is the **regression-detection
+ compliance verification** primitive. "Does this
worker behave the same today as it did when the
auditor sampled it 3 months ago?" — the verdict
trichotomy answers in one chip.

## Scope

- NEW route `/audit/action/<actionId>/replay`.
- On mount: fires `replayAction(actionId)` mutation.
- Loading state: "Replaying… current step: ${n} of
  ${total}" with progress bar.
- Result render:
  - **Replay header** — distinguishing tint +
    "Sandbox — no side effects" banner + verdict
    pill.
  - **Verdict pill** — one of three:
    - `✓ Reproducible` (success tone)
    - `⚠ Drifted-aligned` (warning tone)
    - `⨯ Drifted-misaligned` (destructive tone)
  - **Output diff card** — side-by-side or stacked
    (mobile) original vs replayed.
  - **Provenance diff card** — 5 axes (model /
    prompt / skill / memory / policy) each `same`
    or showing the version delta.
- Failure modes:
  - `Replay unavailable` (decommissioned model).
  - `Replay failed` (sandbox error) — retry +
    report.
  - `Side-effect leak detected` (red banner;
    aborted).
- `[ ↑ Back to action ]` + `[ ⤓ Export replay
  artifact ]` buttons at bottom.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  `web/app/(app)/audit/action/[actionId]/replay/page.tsx`
  (NEW).

## Files you may touch

- `web/app/(app)/audit/action/[actionId]/replay/page.tsx`
  — NEW. ≤ 350 LOC.
- `web/components/skycot/audit/ReplayHeader.tsx` —
  NEW. ≤ 200 LOC.
- `web/components/skycot/audit/VerdictPill.tsx` —
  NEW. Three-tone pill with icon. ≤ 100 LOC.
- `web/components/skycot/audit/ReplayDiff.tsx` —
  NEW. Side-by-side output diff with token-level
  highlighting. ≤ 350 LOC.
- `web/components/skycot/audit/ProvenanceDiffCard.tsx`
  — NEW. 5-axis diff renderer. ≤ 200 LOC.
- `web/components/skycot/audit/ReplayProgress.tsx`
  — NEW. Loading-state progress bar. ≤ 120 LOC.
- `web/lib/api/replay.ts` — NEW.
  `replayAction(actionId)` + result types. Mock
  fallback. ≤ 250 LOC.
- `web/lib/hooks/use-replay.ts` — NEW. TanStack
  Query mutation hook. ≤ 150 LOC.
- `web/lib/sample-replay.ts` — NEW. 3-5 mock replay
  results spanning the verdict trichotomy +
  decommissioned-model + leak scenarios. ≤ 300 LOC.
- `web/app/(app)/audit/action/[actionId]/replay/page.test.tsx`
  — NEW. ≥ 10 tests covering each verdict + failure
  mode. ≤ 400 LOC.

## Files you must NOT touch

- All E1-E4 / earlier-E5 files except read-only.
- `crates/**` — the existing `claude-audit-replay`
  brief handles the backend; this is the UI on top.

## Acceptance criteria

- [ ] Route `/audit/action/<id>/replay` loads.
- [ ] On mount: fires `replayAction(actionId)`
      mutation.
- [ ] **Loading state:**
      - Centered progress block: spinner +
        `"Replaying… step ${current} of ${total}"`.
      - Server emits progress events (or v1 mock
        simulates 4 stages).
- [ ] **Result rendering — Reproducible:**
      - Banner: "Sandbox — no side effects"
        (distinguishing tint).
      - Verdict pill: `✓ Reproducible` (success).
      - Output diff: shows original = replayed
        (no differences highlighted).
      - Provenance diff: 5 axes all `same`.
- [ ] **Result rendering — Drifted-aligned:**
      - Verdict pill: `⚠ Drifted-aligned` (warning).
      - Output diff: side-by-side with token-level
        highlights (added/removed/same).
      - Provenance diff: shows which axes changed
        (e.g. `Skill: v3.0.2 → v3.0.4 (updated 2
        days ago)`).
- [ ] **Result rendering — Drifted-misaligned:**
      - Verdict pill: `⨯ Drifted-misaligned`
        (destructive).
      - Same diff cards as drifted-aligned.
      - Top of page: red banner explaining "Replay
        output would have failed policy. Operator
        notified."
- [ ] **Failure mode — replay unavailable
      (decommissioned model):**
      - Title: `"Replay unavailable"`.
      - Body: `"${modelName} is no longer deployable
        in the sandbox. Original output preserved
        below."`
      - Renders the original output card (read from
        E5-c) for reference.
- [ ] **Failure mode — replay failed:**
      - Title: `"Replay failed"`.
      - Body: error message.
      - `[ Try again ]` retry button.
- [ ] **Failure mode — side-effect leak detected:**
      - Red banner: `"Replay aborted — side-effect
        leak detected. This should never happen. A
        self-audit row has been written. Please
        report to operations."`
      - No diff cards rendered.
- [ ] Footer buttons:
      - `[ ↑ Back to action ]` → routes to
        `/audit/action/<id>`.
      - `[ ⤓ Export replay artifact ]` → opens
        export dialog (E5-e placeholder).
- [ ] Computer-use replay variant: renders
      screenshot sequence comparison (original vs
      replayed) side-by-side, with the click trace.
- [ ] Mock fallback in dev returns one of the
      verdicts based on a hash of the action ID
      (so behavior is deterministic per action).
- [ ] 10 unit tests pass (one per verdict + each
      failure mode + the back/export navigation).
- [ ] Playwright e2e: click Replay on a drill-in,
      see verdict + diff, click Back, return to
      action page.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 10KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E5-d-audit-replay-sandbox`
- PR title: `web(audit): replay sandbox — verdict + diff (E5-d)`

## Notes for the implementer

- **Replay result type** (locked):
  ```ts
  export type Verdict = "reproducible" | "drifted-aligned" | "drifted-misaligned";

  export type ReplayOutcome =
    | { kind: "success"; verdict: Verdict; result: ReplayResult }
    | { kind: "unavailable"; reason: "model-decommissioned"; modelName: string }
    | { kind: "failed"; reason: string }
    | { kind: "leak-detected"; details: string };

  export interface ReplayResult {
    originalOutput: { kind: "text" | "json" | "computer-use"; body: unknown };
    replayedOutput: { kind: "text" | "json" | "computer-use"; body: unknown };
    provenanceDiff: {
      model: { changed: boolean; from?: string; to?: string };
      promptTemplate: { changed: boolean; from?: string; to?: string };
      skill: { changed: boolean; from?: string; to?: string };
      memorySnapshot: { changed: boolean };
      policy: { changed: boolean; from?: string; to?: string };
    };
  }
  ```

- **Token-level diff.** Use a simple word-level diff
  algorithm (no library needed). Each token tagged
  `same` / `add` / `remove`. Render:
  - `same` → plain text
  - `add` → green background (success tone @ 12%
    alpha)
  - `remove` → red background (destructive tone @
    12% alpha)

- **Provenance diff render.** Each of the 5 axes
  in a row:
  ```
  Model:    same
  Prompt:   same
  Skill:    v3.0.2 → v3.0.4 (updated 2 days ago)
  Memory:   same
  Policy:   same
  ```
  Axes with `changed: true` render in warning tone;
  others muted.

- **Verdict logic.** v1 server returns the verdict;
  client just renders. The logic is:
  - `reproducible`: same output AND same provenance.
  - `drifted-aligned`: different output but the new
    output still passes the policy in scope.
  - `drifted-misaligned`: different output AND new
    output would fail current policy.

- **Computer-use replay rendering.** Show two side-
  by-side screenshot grids:
  ```
  Original sequence:    Replayed sequence:
  [s1] [s2] [s3]        [s1'] [s2'] [s3']
                       (any differences marked)
  ```
  Each thumbnail is clickable → lightbox.

- **Mock fallback deterministic.** Hash the action
  ID to a verdict so the same ID always returns
  the same result. v1 sample:
  ```ts
  function mockVerdict(actionId: string): Verdict {
    const h = actionId.charCodeAt(actionId.length - 1) % 3;
    return (["reproducible", "drifted-aligned", "drifted-misaligned"] as const)[h];
  }
  ```

- **Auto-audit.** Each replay is itself audit-
  logged as a row in the audit timeline with
  `is_replay: true`. v1 mock doesn't write the row;
  v1.5 server-side does.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-replay-verdict-granularity`
  — more granular verdict states beyond the
  trichotomy.
- `mission-enterprise-replay-cost-cap` — per-
  tenant cost cap on replays.

Sibling backend brief (already exists):

- `claude-audit-replay` (already in
  MISSION_BACKLOG) — server-side replay sandbox.
