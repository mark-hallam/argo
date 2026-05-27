# ops-supervisor-block-runaway-zombie-claims-on-brief

**Owner**: claude or codex (lightweight bash; either pool can ship).
**Depends on**: none.
**Authored against**: origin/main `cb264f32f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-supervisor-block-runaway-zombie-claims-on-brief`.

**Severity:** P1.
**Source:** docs/skycot/evals/scheduled/2026-05-27/round-102-p1-runaway-zombie-claim-storm.md (R102 P1 NEW recommendation). Originally surfaced as P2 recommendation in R91 (2026-05-26T19:25Z); promoted to P1 in R102 after a second, much larger storm hit (83 zombie cycles on a single brief in 16 minutes).
**Estimated size:** ~80-120 LOC bash + ~40-60 LOC hermetic smoke test.

## Why this brief exists

Workers pick the **first eligible `TODO` row by document order** in `docs/build-plan/BUILD_STATUS.md` (per `docs/build-plan/CONTRIBUTING_AGENT.md` step 2). When a worker fails to ship — for any reason that exits the iteration without opening a PR — `scripts/claude-worker-loop.sh` lines 696-730 correctly demotes the row back to `TODO` with a release commit `status: release <brief-id> (zombie; no PR opened)`. **This part is healthy.**

What is **NOT** healthy: nothing prevents the next worker iteration from immediately re-claiming the same `TODO` row that just zombie-released. The supervisor spawns a new iteration, the new iteration picks `first eligible TODO`, which is still the same brief (it just got demoted back), and the cycle repeats.

This produces a **runaway pattern** where a single brief consumes the full worker pool indefinitely.

### Evidence — R102 active storm (verbatim from the ledger at the time of authoring)

`docs/build-plan/BUILD_STATUS.md:371` — `fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira` ledger row contains 49 consecutive `claim-with-retry` entries between `2026-05-27T13:02:08Z` and `2026-05-27T13:18:59Z` — that is **one claim every 20-30 seconds for 16 minutes**, with **zero PRs opened**:

```text
| fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira | claude | TODO | — |
  claimed via claim-with-retry on mac-host at 2026-05-27T13:18:59Z.
  claimed via claim-with-retry on mac-host at 2026-05-27T13:18:29Z.
  claimed via claim-with-retry on mac-host at 2026-05-27T13:18:12Z.
  ... (49 entries total) ...
  claimed via claim-with-retry on mac-host at 2026-05-27T13:02:08Z.
```

`git log --pretty="%s" origin/main -200 | grep -c "zombie; no PR opened"` returned **99** at the time of authoring; the most-zombied brief accounts for **83 of those 99**. A second brief (`fix-audit-list-kind-query-param-ignored`) is mid-storm in parallel with 12 cycles.

### Why this is P1 not P2

R91 (2026-05-26T19:25Z) recommended this brief at P2 after PR #2429 spawned **~25 zombie cycles in ~64 minutes** before the worker successfully shipped. PR #2429's row was manually cleaned by the conductor that tick:

> **🚨 NOTE**: this brief spawned ~25 worker zombie-claim cycles in ~64min before the worker successfully opened PR #2429; ~170 duplicate RELEASED notes that previously bloated this row have been cleaned in this tick. R91 recommends new P2 brief ops-supervisor-block-runaway-zombie-claims-on-brief (~40 LOC).

18 hours later, an active storm is **3× larger** on a single brief (83 vs ~25 cycles) and has been **running continuously for ~hours** with no terminal state. This is no longer hypothetical or one-off — the failure mode is recurring on the live swarm with steadily worsening severity. Promotion to P1.

### Why the existing zombie-detection doesn't catch this

`scripts/claude-worker-loop.sh` lines 445-547 has a content-aware zombie-detection watchdog that kills wedged worker processes (claude in `SN` nice'd-sleep state with no streaming events). That correctly catches **intra-iteration** zombies — a single worker stuck mid-iteration.

The runaway pattern is a **cross-iteration** failure: each individual iteration is healthy (worker exits cleanly, supervisor spawns a fresh iteration), but the supervisor has no visibility into _which brief_ the previous iteration tried and failed. The enforcement point must be the supervisor, not the worker.

`scripts/worker-supervisor.sh` has zero per-brief logic (`grep -nE "(per.brief|brief.history|claim.count|brief.attempt|throttle)" scripts/worker-supervisor.sh` returns nothing).

## What to build

### File 1 — `scripts/check-runaway-zombie-claims.sh` (NEW, ~50-80 LOC bash)

A standalone check the supervisor invokes between iterations. Exit codes used as a 3-state signal:

```text
Usage:
  bash scripts/check-runaway-zombie-claims.sh [--window-minutes N] [--threshold M] [--block-threshold K]

Exit codes:
  0 — clean (no brief has ≥M zombie-releases in last N minutes)
  2 — throttle (some brief has ≥M but <K zombie-releases — skip claiming it this iter; supervisor picks next-eligible-TODO)
  3 — block-required (some brief has ≥K zombie-releases — supervisor MUST demote row to BLOCKED before next iteration)

Default thresholds:
  --window-minutes 30
  --threshold 3       (skip-claim threshold)
  --block-threshold 6 (force-BLOCKED threshold)

Stdout on non-zero exit (machine-readable):
  brief=<id> count=<N> window=<X>min action=<skip|block>

Stderr on non-zero exit:
  [supervisor-block] brief=<id> reason=runaway-zombie-claims count=<N> window=<X>min action=<skip|block>
```

Implementation outline:

1. `git log --since="$WINDOW_MIN minutes ago" --pretty="%s" origin/main` → grep `^status: release .* (zombie; no PR opened)$` → extract brief-id with `sed`/`awk`.
2. `sort | uniq -c | sort -rn` to get per-brief release counts in window.
3. `awk` over the counts: if max < threshold → exit 0; if max ≥ block-threshold → emit `action=block` line, exit 3; else → emit `action=skip` line, exit 2.
4. Emit machine-readable stdout (one line, key=value pairs) and human-readable stderr.

### File 2 — `scripts/worker-supervisor.sh` integration (~20-30 LOC change)

Insert one check call into the supervisor's per-iteration spawn loop. The supervisor's existing logic finds the worker target slot, then spawns `claude-worker-loop.sh`. Before that spawn:

```bash
# Skip-claim throttle / hard-block on runaway zombie-claim patterns.
# When a single brief has been zombie-released ≥3 times in the last 30 min,
# skip this iteration to give the row a chance to drop off the top-of-queue
# (the next iteration picks the next-eligible TODO). When ≥6 times, the row
# is force-demoted to BLOCKED so manual conductor/operator review is required.
runaway_exit=0
runaway_out=$(bash scripts/check-runaway-zombie-claims.sh) || runaway_exit=$?
case "$runaway_exit" in
  0) ;;  # clean
  2)
    echo "[supervisor] $runaway_out — skipping this iteration to let row drop"
    sleep 60
    continue
    ;;
  3)
    # Parse `brief=<id>` from $runaway_out and flip the row to BLOCKED.
    block_brief=$(echo "$runaway_out" | sed -nE 's/.*brief=([^ ]+).*/\1/p')
    echo "[supervisor] $runaway_out — demoting $block_brief to BLOCKED"
    bash scripts/demote-to-blocked.sh "$block_brief" "runaway-zombie-claim guard"
    continue
    ;;
esac
```

### File 3 — `scripts/demote-to-blocked.sh` (NEW, ~30-40 LOC bash)

Idempotent ledger flip helper that:

1. `git fetch origin main && git pull --ff-only origin main` (fail-safe under concurrent worker pushes).
2. Python `re.sub` regex on `BUILD_STATUS.md` flipping `| <brief-id> | (claude|codex) | TODO |` → `| <brief-id> | $owner | BLOCKED |` and appending salvage note:

   ```text
    **BLOCKED by runaway-zombie-claim guard:** worker exited without
    opening PR ≥6 times in 30 min; manual conductor/operator review
    required before re-claiming. See scripts/check-runaway-zombie-claims.sh.
   ```

3. If the regex matched, `git add docs/build-plan/BUILD_STATUS.md && git commit -m "status: BLOCKED <brief-id> (runaway-zombie-claim guard)" && git pull --rebase origin main && git push origin main`. If regex didn't match (row was already non-TODO — e.g. another worker just claimed it), exit 0 silently — idempotent.
4. The conductor's next tick will see the BLOCKED row in the standard sweep (BLOCKED is not eligible for claim per `CONTRIBUTING_AGENT.md` step 2) and surface it for triage.

### File 4 — `scripts/check-runaway-zombie-claims-tests.sh` (NEW, hermetic smoke test ~40-60 LOC bash)

Hermetic test that creates a throwaway `git init` repo with synthesised commit history and asserts the check fires correctly:

- **Case A — clean:** 0 zombie commits → exit 0, no stderr.
- **Case B — below threshold:** 2 zombie commits same brief, in window → exit 0.
- **Case C — throttle:** 4 zombie commits same brief, in window → exit 2, stderr contains `brief=<id> reason=runaway-zombie-claims count=4 action=skip`.
- **Case D — block:** 7 zombie commits same brief, in window → exit 3, stderr contains `action=block`.
- **Case E — outside window:** 8 zombie commits same brief but all >30min ago → exit 0.
- **Case F — multiple briefs interleaved:** 5 zombies brief X, 2 zombies brief Y → exit 2 with `brief=X` (max-only reporting).

Smoke harness pattern matches `scripts/claim-with-retry-tests.sh` shape (already in tree).

## Acceptance criteria

- [ ] **First criterion (regression turns 🟢)**: `bash scripts/check-runaway-zombie-claims-tests.sh` exits 0 with all 6 cases (A-F above) passing.
- [ ] `scripts/check-runaway-zombie-claims.sh` invoked against a fabricated `git log` with 0 zombie commits exits 0 with no stderr.
- [ ] `scripts/check-runaway-zombie-claims.sh` invoked against a fabricated `git log` with 7 same-brief zombie commits in last 30 min exits 3 with `[supervisor-block] brief=<id> reason=runaway-zombie-claims count=7 window=30min action=block` on stderr.
- [ ] `scripts/demote-to-blocked.sh <brief-id> <reason>` against a checked-out copy of `BUILD_STATUS.md` containing a `TODO` row for that brief flips it to `BLOCKED` and is idempotent (running twice leaves a single BLOCKED row, not two).
- [ ] After integration into `scripts/worker-supervisor.sh`, a synthesised storm in a sandbox `git init` repo (6+ zombies on one brief in 30 min) causes the supervisor's next-iteration spawn loop to invoke `demote-to-blocked.sh` exactly once and continue (not spin).
- [ ] No false positives: a legitimate brief that took 3 iterations to ship (2 zombies + 1 success) over a 2-hour span produces no `[supervisor-block]` lines and no BLOCKED flips.
- [ ] `scripts/worker-supervisor.sh` LOC budget still ≤500 after integration (currently 464 LOC; addition ~20-30 LOC).

## May-touch

- `scripts/check-runaway-zombie-claims.sh` (NEW, ~50-80 LOC bash).
- `scripts/check-runaway-zombie-claims-tests.sh` (NEW, hermetic harness, ~40-60 LOC bash).
- `scripts/demote-to-blocked.sh` (NEW, ~30-40 LOC bash).
- `scripts/worker-supervisor.sh` (~20-30 LOC ADD; total file must stay ≤500 LOC — currently 464).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open — worker's responsibility, not the brief's code).

## Out of scope (do NOT widen)

- Investigating WHY a specific brief (e.g. `fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira`) keeps failing to ship — that is a per-brief root-cause investigation owned by the conductor when reviewing the BLOCKED row. The supervisor guard is brief-agnostic.
- Modifying the worker-loop's existing zombie-detection logic in `scripts/claude-worker-loop.sh` (lines 445-547 and 696-730) — that logic is load-bearing for legit zombie cleanup and stays untouched.
- Changing `scripts/claim-with-retry.sh` behaviour.
- Stopping the **currently-running storm** at the time of authoring — the conductor or operator can manually flip the two storm rows to BLOCKED in a separate tick.
- Adding zombie-claim observability to dashboards / Slack / SHARED.md — the BLOCKED row + status commit is the surface.
- Auto-unblocking BLOCKED rows after some cooldown — manual conductor/operator triage is the v1 model.

## Parallel-surface check

```text
$ rg 'pub (struct|enum|type) ' scripts/
(empty — bash, no Rust types)

$ rg "runaway-zombie|runaway_zombie|zombie-runaway|zombie_runaway" scripts/ docs/build-plan/streams/
(empty — no existing surface with this name)

$ ls scripts/check-runaway-zombie-claims*.sh scripts/demote-to-blocked.sh 2>/dev/null
(none exist — all three are NEW)

$ ls docs/build-plan/streams/ | grep -iE "supervisor-block|runaway|claim-throttle|throttle-claim|max-claim|claim-limit"
(none — this brief-id is unique)
```

Related (NOT colliding) briefs in tree:

- `ops-worker-supervisor-detect-zombie-wait-loop-pattern.md` (IN_REVIEW as PR #2278) — content-aware **intra-iteration** wait-loop detection (`^Waiting for clippy/tests to finish`). Different defect: that brief kills workers stuck in a single iteration emitting health pulses; this brief catches **cross-iteration** runaways where each iteration exits cleanly.
- `claude-worker-loop-zombie-detection.md` + `claude-worker-loop-zombie-detection-followup.md` (MERGED) — the in-worker `SN`-state wedge detector. Different layer.

These three coexist cleanly: worker-loop kills wedged workers, the IN_REVIEW PR #2278 kills wait-loop'd workers, and this brief stops the supervisor from re-claiming a brief that keeps zombie-releasing across iterations.

## Source

docs/skycot/evals/scheduled/2026-05-27/round-102-p1-runaway-zombie-claim-storm.md (R102 P1 finding + brief recommendation). Promoted from R91's P2 recommendation (docs/skycot/evals/scheduled/2026-05-26/round-91-*.md and SHARED.md handoff `2026-05-26T19:25:00Z [evaluator] round 91`).
