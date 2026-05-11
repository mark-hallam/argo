# Stream E4-h — Worker profile: AddResponsibilityDialog

**Owner**: claude.
**Depends on**: E0 MERGED; E4-a, E4-b MERGED (DutiesCard hosts the entry point); E4-g MERGED (reuses the classifier + commit pattern).
**Authored against**: origin/main `9fd8f587` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-h-worker-add-responsibility-dialog`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 brief 8 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Classification scope | Genesis-lite classifies the request as memory/policy/skill/vocabulary/duty (reuses E4-g classifier) | Same | n/a |
| Duty conflict detection | Worker-level cron conflicts only | Cross-worker / cross-team duty-scheduling conflicts (don't schedule Mariah's vendor-renewal cron at the same hour as Sam's invoice-close cron if they share a downstream system) | `mission-enterprise-duty-cross-worker-conflicts` |
| Cron expression authoring | Tiny client cron parser (E2-f) + 5 preset chips + custom | Server-side validation + cron-string semantic preview ("Every weekday at 9am — next fire Tuesday 2026-05-13") | `mission-enterprise-cron-semantic-preview` |
| Pattern detection trigger path | Manual click only via DutiesCard | Pattern detector watches the audit log + surfaces "you've approved this 30× — want Mariah to handle autonomously?" on the Approvals inbox | `mission-enterprise-pattern-detection-surface` |
| Cost projection | None | Per-duty cost estimate before commit (LLM calls × frequency × per-call cost) | `mission-enterprise-duty-cost-projection` |
| Multi-stakeholder approval | Single delegator commits | Department-level approval for duties that touch financial/legal/HR data | `mission-enterprise-duty-multi-approval` |

v1 is acceptable: AddResponsibility committing at the
worker level is the SMB-path; cross-team conflict
detection + multi-approval are Track-H.

## Why

Per [journey-delegate-ongoing-work.md](../../skycot/ux-research/journey-delegate-ongoing-work.md),
the user adds a new responsibility to an existing
teammate via plain English. E4-b's DutiesCard has a
"+ Add responsibility" button that opens this dialog.

This brief implements the 6-step UX flow (same shape
as E4-g RetrainDialog, scoped to duty additions).

## Scope

- `AddResponsibilityDialog` opens from E4-b's
  DutiesCard "+ Add responsibility" button.
- 6-step flow:
  - **Step 1 — Capture**: free-text textarea + cron-
    preset chips ("Every morning", "Every hour",
    "Weekdays 9am", "Once a week (Mon)", "Custom...").
  - **Step 2 — Classify**: Genesis-lite classifies
    into duty-shape (recurring? event-triggered?
    one-shot?).
  - **Step 3 — Draft**: proposed duty in plain
    English + cron expression preview + 3 sample
    outputs the duty would produce.
  - **Step 4 — Conflict resolution** (skipped if no
    conflict): show existing duties this would
    overlap with; pick Replace / Keep both / Cancel.
  - **Step 5 — Commit**: `[ ✓ Add this duty ]`.
  - **Step 6 — Confirmation + reverse-test offer**:
    `"Want me to re-run this on the last 3 cases?
    Nothing gets sent."` (similar to E4-g pattern).
- DutiesCard updates with the new duty after commit.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/components/skycot/worker/
  DutiesCard.tsx` (E4-b — read-only, this brief
  swaps the placeholder for the real dialog mount).

## Files you may touch

- `web/components/skycot/worker/AddResponsibilityDialog.tsx`
  — NEW. 6-step dialog. ≤ 450 LOC.
- `web/components/skycot/worker/duty/Step1Capture.tsx`
  — NEW. ≤ 200 LOC.
- `web/components/skycot/worker/duty/Step2Classify.tsx`
  — NEW. ≤ 120 LOC.
- `web/components/skycot/worker/duty/Step3Draft.tsx`
  — NEW. ≤ 280 LOC.
- `web/components/skycot/worker/duty/Step4Conflict.tsx`
  — NEW. ≤ 200 LOC.
- `web/lib/api/duties.ts` — NEW.
  `classifyDuty(input)`, `commitDuty(payload)`,
  `previewDuty(payload)`. Mock fallback. ≤ 250 LOC.
- `web/lib/hooks/use-duty-add.ts` — NEW. ≤ 180 LOC.
- `web/components/skycot/worker/DutiesCard.tsx` —
  narrowly scoped: swap the toast placeholder for
  the real dialog mount. ≤ 320 LOC after.
- `web/components/skycot/worker/AddResponsibilityDialog.test.tsx`
  — NEW. ≥ 10 tests. ≤ 400 LOC.

## Files you must NOT touch

- All E1/E2/E3 / E4-a/c/d/e/f/g files except the
  narrow DutiesCard mount.
- `web/lib/api/genesis.ts` (E2-c) — read-only.
- `web/lib/cron/parse.ts` (from E2-f) — reused.
- `crates/**` — sibling backend brief for duty
  endpoints.

## Acceptance criteria

- [ ] Dialog opens from `+ Add responsibility`
      button in DutiesCard.
- [ ] **Step 1 Capture:**
      - Textarea with placeholder `"What else do you
        want ${workerName} to handle?"`
      - 5 cron-preset chips below:
        - `Every morning` → `0 6 * * *`
        - `Every hour` → `0 * * * *`
        - `Weekdays 9am` → `0 9 * * 1-5`
        - `Once a week (Mon)` → `0 9 * * 1`
        - `Custom...` → opens raw cron input
      - `[ Continue → ]` enabled at ≥ 10 chars in
        textarea + a chip selected OR custom cron
        valid.
- [ ] **Step 2 Classify:**
      - Spinner + `"Figuring out the shape of this
        responsibility..."`
      - 200-500ms simulated; real call goes to
        `classifyDuty` API.
- [ ] **Step 3 Draft:**
      - Plain-English proposed duty.
      - Cron string with semantic preview (e.g.
        `"0 9 * * 1-5 — Every weekday at 9am NZST.
        Next fire: Tuesday 2026-05-13 at 09:00."`)
      - 3 sample outputs the duty would produce on
        recent cases.
      - `[ ✎ Adjust the wording ]` button → returns
        to Step 1 with current text editable.
      - `[ Continue → ]` advances to Step 4.
- [ ] **Step 4 Conflict resolution** (only when
      conflicts detected):
      - Shows existing duty + new duty side-by-side.
      - Radio options: Replace / Keep both / Cancel.
      - `[ Continue → ]`.
- [ ] **Step 5 Commit:**
      - `[ ✓ Add this duty ]` primary button.
      - On click: mutation fires; spinner; success
        → dialog closes.
- [ ] **Step 6 Confirmation toast:**
      - `"Added the new responsibility. ${oneLine}.
        Mariah's next ${preset} run kicks off
        ${nextFire}."`
      - `[ Re-run on recent cases ]` → opens reverse-
        test modal.
- [ ] DutiesCard refreshes with new duty (optimistic
      update).
- [ ] RecentChangesPanel (from E4-g) shows the
      addition as a new entry.
- [ ] Mock fallback in dev for all mutations.
- [ ] 10 unit tests pass.
- [ ] Playwright e2e: open dialog from DutiesCard,
      type a duty, advance through steps, assert
      duty appears.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 9KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-h-worker-add-responsibility-dialog`
- PR title: `web(workers): AddResponsibilityDialog — duty add flow (E4-h)`

## Notes for the implementer

- **Reuse E4-g's step-machine pattern.** Internal
  state `currentStep: 1|2|3|4|5|6` with conditional
  Step 4. Step 6 happens after dialog closes via
  toast.

- **Cron preset chips.** Render as toggle pills below
  the textarea. Single-select (clicking one
  deselects others). "Custom..." opens an inline
  cron input.

- **Cron semantic preview.** Use the existing
  `parseCron` from E2-f's `lib/cron/parse.ts`. The
  parser already converts cron strings to human-
  readable form. Extend if needed with "Next fire:
  X" by computing next-fire timestamp client-side
  (use `cron-parser` package — but check first if
  E2-f added it; otherwise inline a simple
  computation).

- **3 sample outputs.** Server-side classifier
  returns synthesized samples per the duty's intent.
  v1 mock returns 3 hardcoded samples per
  classification type.

- **Conflict detection** in v1 — worker-level only:
  - Check the worker's existing duties for cron
    overlaps (same minute + same hour + day
    overlap).
  - Surface only direct overlaps, not "close
    enough" conflicts.

- **Reverse-test modal copy:**
  ```
  "Here's how Mariah would have handled these
  3 recent cases with the new duty applied —
  nothing was actually sent.

  Case 1: [inbound context] → [duty output]
  Case 2: ...
  Case 3: ...

  Looks good? The duty is already live; this
  is just preview.

  [ Close ]"
  ```

- **Duty data shape (locked):**
  ```ts
  interface Duty {
    id: string;
    label: string;
    schedule: { kind: "cron" | "event"; spec: string };
    description: string;          // plain English
    enabled: boolean;
    nextFireAt?: string;
    createdAt: string;
    createdBy: string;
  }
  ```

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-duty-cross-worker-conflicts` —
  cross-worker / cross-team conflict detection.
- `mission-enterprise-cron-semantic-preview` —
  server-side semantic preview.
- `mission-enterprise-pattern-detection-surface` —
  pattern detector surfacing in approvals inbox.
- `mission-enterprise-duty-cost-projection` — pre-
  commit cost estimate.
- `mission-enterprise-duty-multi-approval` —
  department-level approval for high-stakes
  duties.

Sibling backend brief:

- `mission-gateway-duty-endpoints` — classify +
  commit + preview duty endpoints.
