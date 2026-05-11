# Stream E4-g — Worker profile: RetrainDialog (plain-English correction → durable change)

**Owner**: claude.
**Depends on**: E0 MERGED; E4-a..f MERGED.
**Authored against**: origin/main `8b41f643` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-g-worker-retrain-dialog`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 brief 7 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Classifier accuracy | Server-side Genesis-lite classifier (per `mission-genesis-llm-classifier` MERGED) classifies into 5 categories; client renders proposal | Same; v1 ships the existing classifier wiring | n/a |
| Multi-stakeholder approval | Single delegator commits | Department-level retrains route through department-lead approval; per-tenant policy on approval depth | `mission-enterprise-retrain-multi-approval` |
| Conflict detection scope | Worker-level only | Cross-worker + cross-team conflict detection (e.g. retraining Mariah's vendor rules conflicts with Sam's AP rules — surface the conflict before commit) | `mission-enterprise-retrain-cross-worker-conflicts` |
| Regression test scope | 50 recent actions (from `mission-skill-regression-detector` MERGED) | Plus a curated test set per vertical (operator-supplied "golden examples") | `mission-enterprise-retrain-golden-examples` |
| Undo TTL | 14 days client + server | Same; v1 ships both | n/a |
| Audit visibility | Single audit row per retrain | Full retrain event bundle: input → classify → preview → conflict → regression → commit, exportable | `mission-enterprise-retrain-audit-bundle` |

v1 is acceptable: retrain mutations all run through
existing classifier + regression-detector infrastructure;
the user-facing flow is the missing piece.

## Why

The retrain journey doc
([journey-retrain-a-worker.md](../../skycot/ux-research/journey-retrain-a-worker.md))
specifies an 8-step flow: initiate → classify →
propose plain English → conflict/generalization →
regression check → commit → reverse-test → trust-
ratchet reset. E4-b and E4-d stub the entry points
with placeholders ("Adjust voice in E4-g"); this
brief ships the real dialog.

For enterprise: corrections at workers may affect
department-wide behavior. v1 still commits at the
worker level only; cross-team detection is Track-H
follow-up.

## Scope

- `RetrainDialog` opens from:
  - E4-b's WhatSheDoesCard "✎ Adjust" button (charter
    correction).
  - E4-d's VoiceProfileSummary "✎ Adjust" button (voice
    correction).
  - E5's audit drill-in "Make sure this doesn't happen
    again" (link from E5-c — not in this PR; just
    accept the prop).
- 6-step UX flow inside the dialog:
  - **Step 1 — Capture**: free-text textarea (≥ 4
    rows). "What do you want Mariah to do differently?"
  - **Step 2 — Classify** (200-500ms wait): spinner +
    "Figuring out the right place for this..."
  - **Step 3 — Propose**: rendered output of the
    classifier, plain English (per journey-retrain
    Step 3 template). Includes "Try it on 3 recent
    cases" button.
  - **Step 4 — Conflict resolution** (skipped if no
    conflict): side-by-side existing-vs-new, pick
    one.
  - **Step 5 — Regression check** (skipped if no
    regressions): preview of changed outputs on
    prior cases.
  - **Step 6 — Commit**: "✓ Train it in" button
    triggers mutation; success → close dialog +
    add a "Recent changes" entry on the worker
    profile (E4-b's overview tab refreshes).
- After-commit: reverse-test offer in toast: `"Want
  me to re-handle 3 recent conversations with the
  new rule? Nothing gets sent."` →
  `[ Show me ]` opens a modal with the
  before/after previews.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/components/skycot/worker/
  WhatSheDoesCard.tsx`,
  `VoiceProfileSummary.tsx` (read-only — this brief
  swaps the placeholder toast for the real dialog
  via prop-drilling).

## Files you may touch

- `web/components/skycot/worker/RetrainDialog.tsx` —
  NEW. The dialog component with all 6 steps. ≤ 450
  LOC.
- `web/components/skycot/worker/RetrainStepCapture.tsx`
  — NEW. ≤ 150 LOC.
- `web/components/skycot/worker/RetrainStepClassify.tsx`
  — NEW. ≤ 120 LOC.
- `web/components/skycot/worker/RetrainStepPropose.tsx`
  — NEW. ≤ 250 LOC.
- `web/components/skycot/worker/RetrainStepConflict.tsx`
  — NEW. ≤ 220 LOC.
- `web/components/skycot/worker/RetrainStepRegression.tsx`
  — NEW. ≤ 220 LOC.
- `web/lib/api/retrain.ts` — NEW.
  `classifyRetrain(input)`, `commitRetrain(payload)`,
  `previewReverseTest(retrainId)`. Mock fallback. ≤
  250 LOC.
- `web/lib/hooks/use-retrain.ts` — NEW. ≤ 200 LOC.
- `web/components/skycot/worker/RecentChangesPanel.tsx`
  — NEW. Appears on the Overview tab's bottom (under
  the 4 cards). Lists recent retrain entries with
  undo for 14 days. ≤ 200 LOC.
- `web/components/skycot/worker/WhatSheDoesCard.tsx`
  — narrowly scoped: swap the toast placeholder for
  the real RetrainDialog mount. ≤ 280 LOC after.
- `web/components/skycot/worker/VoiceProfileSummary.tsx`
  — narrowly scoped: same swap. ≤ 200 LOC after.
- `web/app/(app)/workers/[workerId]/overview/page.tsx`
  — narrowly scoped: render `<RecentChangesPanel>`
  at the bottom. ≤ 300 LOC after.
- `web/components/skycot/worker/RetrainDialog.test.tsx`
  — NEW. ≥ 12 tests (one per step + happy-path
  end-to-end + conflict path + regression path).
  ≤ 450 LOC.

## Files you must NOT touch

- All E1/E2/E3 / E4-a-c-e-f files — read-only
  consumption.
- `web/lib/api/genesis.ts` (from E2-c) — read-only;
  reuse the classifier function or extend narrowly
  if needed.
- `crates/**` — sibling backend brief for the
  classify/commit endpoints.

## Acceptance criteria

- [ ] RetrainDialog opens from E4-b's "✎ Adjust"
      button on WhatSheDoesCard.
- [ ] RetrainDialog opens from E4-d's "✎ Adjust"
      button on VoiceProfileSummary.
- [ ] Dialog renders the 6-step flow:
      - Step 1: free-text textarea ≥ 4 rows + `[
        Continue → ]` (disabled until ≥ 10 chars).
      - Step 2: spinner + classification call (200-
        500ms simulated in v1 mock).
      - Step 3: proposed change in plain English +
        `[ Try it on 3 recent cases ]` button.
      - Step 4: surfaces conflicts when detected;
        offers Replace / Keep both / Cancel.
      - Step 5: regression preview when applicable.
      - Step 6: `[ ✓ Train it in ]` commits.
- [ ] Try-it-on-3-cases: opens a side panel showing
      3 sample inbound contexts + how the worker
      would respond with the new rule applied.
- [ ] Commit:
      - Mutation fires → success → dialog closes.
      - Toast: `"Trained Mariah on this. Going
        forward, ${oneLineSummary}. ${[ See in
        Recent changes ]}"`.
      - RecentChangesPanel updates to show the new
        entry at the top.
- [ ] Reverse-test offer (in toast): `[ Re-handle
      recent cases ]` → opens modal with before/
      after pairs.
- [ ] **RecentChangesPanel** at the bottom of Overview
      tab:
      - Lists last 14 days of retrains, newest first.
      - Per row: relative time + 1-line summary +
        `[ Undo ]` button (if within 14 days).
      - Undo: confirm dialog → mutation → entry
        flagged "undone" with a strikethrough.
- [ ] Mock fallback for all retrain mutations in
      dev.
- [ ] 12 unit tests pass.
- [ ] Playwright e2e: visit Overview, click Adjust,
      type a correction, click through 6 steps,
      verify Recent changes panel updated.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 10KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-g-worker-retrain-dialog`
- PR title: `web(workers): RetrainDialog — plain-English correction flow (E4-g)`

## Notes for the implementer

- **Step machine.** Internal state in
  `RetrainDialog`: `currentStep: 1|2|3|4|5|6`. Each
  step component receives `{ state, advance, back }`
  callbacks. Steps 4 + 5 are conditional — Step 3's
  classifier response includes `hasConflict?` and
  `hasRegressions?` flags that drive whether those
  steps appear.

- **Classifier output shape:**
  ```ts
  interface RetrainProposal {
    classification: "memory" | "policy" | "skill" | "vocabulary" | "action-ban";
    plainEnglish: string;         // the rendered description
    conflicts?: ConflictDescriptor[];
    needsGeneralization?: { question: string; options: string[] };
    regressionPreview?: RegressionDiff[];
    structuralChange: {
      kind: string;
      payload: unknown;            // depends on classification
    };
  }
  ```

- **Try-it-on-3-cases sample data.** Real
  implementation queries audit-rows from the worker's
  past 30 days that match the proposed rule's
  predicate. v1 mock: 3 hardcoded inbound contexts
  per classification kind.

- **Conflict resolution copy:**
  ```
  This contradicts: '${existingRule}' (added ${date}).

  ${newRule} would override the existing rule for these
  cases.

  [ ] Replace the old rule with the new one
  [ ] Keep both — the new rule wins for ${specificCases}
  [ ] Cancel this change
  ```

- **Regression preview component.** Shows 3-5 cases
  where the new rule changes the worker's behavior:
  - "Before": original output.
  - "After": output if new rule applied.
  - Outcome tag: "safe" (policy compliant) /
    "review" (policy concern) / "blocked" (would
    violate higher-level policy).
  Per row: a toggle to accept/reject per-case;
  Commit applies the new rule to all "safe" cases
  only, surfaces "review" + "blocked" for explicit
  operator handling.

- **RecentChangesPanel UX.** Subtle visual; lives
  at the bottom of the Overview tab. Each row a
  thin one-line summary:
  ```
  • 3m ago — Added rule: "no PO box shipping"
                                          [ Undo ]
  • 1d ago — Voice: tone changed to formal
                                          [ Undo ]
  • 4d ago — Memory: "use 'guests' not
            'customers'"        (undone 1d ago)
  ```
  Undone entries strikethrough + show the
  undo-timestamp.

- **Undo behavior.** Server-side endpoint must
  support reversal (uses the audit log's snapshot).
  v1 mock: marks the entry undone in client state
  + audit row appended.

- **Trust-ratchet reset.** When a policy retrain
  commits, the affected action type's trust
  counter resets to 0. v1 mock: state-only; v1.5
  server-enforced. Audit row records the reset.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-retrain-multi-approval` —
  department-level retrain approval workflow.
- `mission-enterprise-retrain-cross-worker-conflicts`
  — cross-worker / cross-team conflict detection.
- `mission-enterprise-retrain-golden-examples` —
  operator-supplied test set per vertical.
- `mission-enterprise-retrain-audit-bundle` — full
  retrain event bundle for compliance.

Sibling backend brief:

- `mission-gateway-retrain-endpoints` — classify
  + commit + undo + reverse-test endpoints.
