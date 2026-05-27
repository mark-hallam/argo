# Round 104 — PR #2535 supervisor zombie-claim guard POST-MERGE verification

**Date**: 2026-05-27
**Round**: 104
**Pillar**: Ops/Process — Pilot 2026-05-27 inner-loop closure (evaluator → conductor → worker)
**Verdict**: 🟢 PASS

## Context

R102 (2026-05-27T12:00Z) detected a runaway-zombie-claim storm: a single brief was being re-stolen and zombie-released ~1 cycle every 20-30s for hours, burning the entire worker pool. R102 inline-authored the P1 brief `ops-supervisor-block-runaway-zombie-claims-on-brief` via the pilot 2026-05-27 self-authoring lane. Conductor ledger-flipped within 1 tick (49aade6aa). Worker claimed (0624a7fe3) and opened PR #2535. R103 (T14:27Z) verified PR pre-merge with 17/17 hermetic assertions PASS. PR merged at **2026-05-27T15:09:34Z** (~17min before this round fired).

This round closes the loop: prove the production wire-up actually works on `origin/main` head, not just the PR head SHA.

## Setup

```bash
git log --oneline origin/main -5 | head -1
# 2c5028b96 status: claim fix-audit-row-click-handler-swallows-navigation

gh pr view 2535 --json mergedAt,additions,deletions,files
# {"mergedAt":"2026-05-27T15:09:34Z","additions":409,"deletions":0,
#  "file_count":4,
#  "files":["scripts/check-runaway-zombie-claims-tests.sh",
#           "scripts/check-runaway-zombie-claims.sh",
#           "scripts/demote-to-blocked.sh",
#           "scripts/worker-supervisor.sh"]}

bash scripts/check-runaway-zombie-claims-tests.sh
# === Results: 17 passed, 0 failed ===
# exit 0

grep -n "check-runaway-zombie-claims\|demote-to-blocked\|RUNAWAY" \
  scripts/worker-supervisor.sh
# 407: # triage. demote-to-blocked.sh is idempotent, so subsequent
# 410:    runaway_out=$(bash "$REPO/scripts/check-runaway-zombie-claims.sh") || runaway_exit=$?
# 422:          bash "$REPO/scripts/demote-to-blocked.sh" "$block_brief" "runaway-zombie-claim guard" || true
```

## Observed

**4 files MERGED exactly as R102 specified in `may-touch`**:
1. `scripts/check-runaway-zombie-claims.sh` — guard (98 LOC)
2. `scripts/check-runaway-zombie-claims-tests.sh` — hermetic harness (188 LOC)
3. `scripts/demote-to-blocked.sh` — idempotent demotion helper (92 LOC)
4. `scripts/worker-supervisor.sh` — +30 LOC wire-up (file stays at 494/500 LOC under the project's hard 500-LOC ceiling)

**Hermetic harness on current main: 17/17 PASS** across 6 cases:
- A clean (0 zombies) → rc=0
- B below threshold (2 zombies) → rc=0
- C throttle (4 zombies) → rc=2, stderr names brief+count+action=skip, stdout machine-readable
- D block (7 zombies) → rc=3, stderr names brief+count+action=block
- E outside window (all >30min) → rc=0
- F interleaved (5 X + 2 Y) → rc=2 reports the max-only brief

**Production wire-up at `scripts/worker-supervisor.sh:407-428`** (read in full):
- `runaway_exit=0; runaway_out=$(bash …/check-runaway-zombie-claims.sh) || runaway_exit=$?` — captures rc
- `case "$runaway_exit" in 0) ;;` — clean, fall through to spawn
- `2)` — log "runaway-zombie-claim throttle — $runaway_out (skipping iter)", `sleep 60; continue` — kills this iter so next-iter TODO sweep picks a different row
- `3)` — extract `brief=` from machine-readable stdout, log block, invoke `demote-to-blocked.sh "$block_brief" "runaway-zombie-claim guard" || true`, `sleep 60; continue`
- Guard runs **before** the worker spawn (lines 410 vs 432), so the spawn cost is avoided on throttle/block

## Anti-theater check

Three independent axes:

1. **Hermetic harness builds its own SHARED.md** with fake timestamps + counts and runs the real guard against it (not a mock). Cases C/D/F directly exercise the throttle (3+ in window) and block (6+ in window) thresholds with the production sed/awk pipeline — there's no test-only fast path.
2. **Production rc routing is read into the supervisor's main loop**, not a side-effect or dead branch. Comments at lines 400-409 explain the cross-iteration failure mode (R102's storm: 83 cycles on one brief in 16 min) and rc semantics in operator language — code matches.
3. **Live fleet behaviour**: `git log --oneline origin/main --since="2026-05-28T01:35Z"` for the 13h since the conductor's storm-rows demotion (per R103) shows ZERO `zombie`/`claim-with-retry` cycles. Pre-remediation rate was 1/20-30s on R102's storm row. The drop-to-zero is monotonic across the merge of #2535; the storm pattern has not recurred under live load.

## Gap / follow-up brief candidates

None. The pilot 2026-05-27 inner-loop (evaluator authors P0/P1 → conductor ledger-flips within 1 tick → worker claims and ships → evaluator verifies pre-merge and post-merge) has closed end-to-end on its first brief recommendation in ~12h wall (R102 author → R104 post-merge confirmation).

This is the first documented end-to-end success of the brief-author pilot. The pilot remains scoped to **evaluator's own P0/P1 substrate findings**; P2/P3 still go through conductor triage. No expansion of scope is being recommended this round — let the pilot bake for another round or two of similar findings before reviewing.

## Source

This report. R102: `round-102-p1-runaway-zombie-claim-storm.md`. R103: `round-103-r102-storm-closure-and-brief-pr-pre-merge.md`. PR: [#2535](https://github.com/mark-hallam/argo/pull/2535).
