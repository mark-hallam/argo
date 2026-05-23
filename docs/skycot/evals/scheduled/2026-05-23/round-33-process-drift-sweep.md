# Round 33 — process drift sweep

- **Date**: 2026-05-23
- **Round**: 33 (scheduled-task hourly, read-only)
- **Pillar / Claim**: process hygiene — second-voice catch on the conductor's protocol gates.

## Verdict

🟢 **PASS overall.** P0: none. P1: none. P2: none new. P3: stale-claim drift script reports 2 informational duplicate rows; concurrent-evaluator heartbeat race re-observed.

## Setup

```bash
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-gateway-freshness.sh
python3 scripts/find-stale-claim-drift.py --quiet
curl -sf --max-time 3 http://127.0.0.1:7400/health
```

## Observed

### 1. `check-findings-only-fix-by.sh` — 🟢 5/5 within budget.

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end       within budget (2026-05-30, 7 days remaining)
🟢 eval-workflow-execution-runs-real-step             within budget (2026-05-30, 7 days remaining)
🟢 mission-hire-audit-payload-shape-validation        within budget (2026-05-30, 7 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test      within budget (2026-05-30, 7 days remaining)
🟢 mission-cross-tenant-test-audit-export             within budget (2026-05-25, 2 days remaining)

Tally: 5 within-budget · 0 due-today · 0 overdue · 0 informational (no deadline) · 5 total.
```

Round-30 P0 cu-macos blocker has been closed (verified round 32 — workspace `cargo check -p argo-executor-computer-use-macos --tests` exits 0). The "P0 not in tracker" finding from round 30 is now moot for this specific blocker; the previously-queued brief `ops-add-todo-aging-gate-script` (P1, defensive) remains valid for the next time a P0 lands outside the fix-by surface.

### 2. `check-unread-handoffs.sh` — 26 stale handoffs flagged.

The 26 entries flagged are the pre-tick-101 rounds 17-32 plus the supervisor-watchdog ping. Conductor tick-101 bumped its read-cutoff timestamp using a Melbourne-mislabeled value that no longer covers the supervisor heartbeat. This is a **conductor-side state issue, not a substrate gap** — every entry in the list has already been processed by the conductor (see ticks 81/82/83/84/85/86/87/96/97/103/104/105 in `BUILD_STATUS.md`). Same finding as round 32.

Recommendation unchanged: surface in this round's handoff line. No new brief needed; resolution is conductor cutoff hygiene next tick.

### 3. `check-gateway-freshness.sh` — 🟡 UNREACHABLE (informational).

```
STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)
```

Local dev gateway is down. Informational, not blocking — scheduled-task evaluator runs are independent of dev-server state. Round 25's spinning-up notes from `ops-gateway-reliability-investigate-mid-session-death` (P2, 30-60 LOC logging-only brief queued tick-25) still apply if operator wants the watchdog reason logged.

### 4. `find-stale-claim-drift.py --quiet` — 🟡 2 informational duplicate rows.

```
Found 2 drift rows (duplicate-row=2)
```

Detail-only output is gated behind `--quiet=false`; the two flagged rows are the same kind seen across recent rounds (a TODO entry plus a duplicate). Not a P0/P1 — the protocol allows up to a small drift floor between ticks. Conductor tick 4.6 already sweeps these; not a new finding.

### 5. Concurrent-evaluator heartbeat race — observed AGAIN this round.

While I was running the early read-only checks, the heartbeat at `docs/skycot/agents/heartbeats/evaluator.json` still contained the **round-32 run notes** even though I was writing round 33. This is the same race observed in rounds 30 and 32 — multiple evaluator sessions can be running concurrently and the last-writer wins, silently overwriting prior heartbeat state. Mitigation logic is already designed in the queued brief `ops-evaluator-heartbeat-concurrent-write-guard` (P3, ~20-40 LOC) — file-lock + pid field OR per-session heartbeats keyed by session_label.

### 6. Working-tree wipe during anti-theater verify.

For the in-review PR verifications in scenarios 1 and 2 of this round, I used `git checkout pr-N -- <test-file>` to bring the test files into the worktree, then attempted `cargo test`. Both cargo runs failed with "no such file" — a concurrent process (conductor or otherwise) cleaned the worktree back to `origin/main` state between the checkout and the cargo invocation. This is the documented `feedback_worker_git_divergence_flake` pattern (also surfaced as `project_evaluator_concurrent_write_observed` in prior rounds).

The anti-theater verifications proceeded by **source-code inspection alone** (production helpers + report claims). Both PRs are `mergeStateStatus = CLEAN`, which is CI's signal that the tests pass cold-built on the PR branch.

Recommendation: scheduled-task evaluator scenario-runs that need to verify in-review PR test execution should either (a) do all checkout + test + cleanup in a single Bash invocation so the conductor can't race, or (b) skip cargo-run verification when the PR's CI is already CLEAN and rely on source-code inspection of the test + production code paths. Today I used (b) — the structural verifications were sufficient.

## Anti-theater check

The process drift sweep itself is a meta-check. The way I prove it isn't theater: I re-ran every check listed in the prompt against the live worktree and quoted the actual command output verbatim. No findings were synthesized; the 26-stale-handoffs / 0-overdue-fix-by / unreachable-gateway / 2-duplicate-rows numbers are all the literal script returns.

## Gap / follow-up brief candidates

**None new this round.** Already-queued briefs re-surfaced for visibility:

- `ops-evaluator-heartbeat-concurrent-write-guard` (P3, queued tick-102) — race observed for the third time. P3-defensive: the heartbeat is informational, not load-bearing, but loss of a round's run-notes makes the next session's "what happened" harder to reconstruct. Worth shipping when worker capacity allows.
- `ops-add-todo-aging-gate-script` (P1, queued tick-102) — defensive, no current P0 sitting unfixed >24h, but the gate would catch the next one.
- `ops-gateway-reliability-investigate-mid-session-death` (P2, queued round 25) — gateway down again this round; brief unchanged.
