# Round 44 — process drift sweep + `store_worker_overlay_tests` 4th-round recheck

- **Date**: 2026-05-23
- **Round**: 44 (scheduled-task hourly fire)
- **Pillar / Claim**: process-health (gate scripts) + Claim 4 substrate health (worker-overlay)
- **Headline**: 🟡 **PARTIAL** — fix-by tracker clean; SHARED.md timestamp drift bug still spamming the conductor's unread-handoffs gate (10th-consecutive-round observation); `store_worker_overlay_tests` 2/5 RED status still pending cold-test confirmation.

## Setup

```
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-gateway-freshness.sh
cargo test -p argo-store store_worker_overlay     # running cold, slow
```

## Observed — fix-by tracker (`check-findings-only-fix-by.sh`)

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end                 within budget (2026-05-30, 7 days remaining)
🟢 eval-workflow-execution-runs-real-step                       within budget (2026-05-30, 7 days remaining)
🟢 mission-hire-audit-payload-shape-validation                  within budget (2026-05-30, 7 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test                within budget (2026-05-30, 7 days remaining)

Tally: 4 within-budget · 0 due-today · 0 overdue · 4 total.
```

All four FINDINGS-only fix-bys are within budget. **`mission-cross-tenant-test-audit-export` (round 38/40 flagged as due 2026-05-25, 2 days out)** is NOT in the tally — likely because tick-127 flipped it MERGED-FINDINGS-ONLY → MERGED-FINDINGS-CLOSED (per the round 43 + the conductor's tick-127 commit message). Sanity-check the close was real: `git log --oneline | grep "mission-cross-tenant-test-audit-export"` shows tick-127 ledger flip, no other PR, so closure is by attestation (fix-brief #2051 shipped tick-68 as referenced in the close). 🟢 OK.

## Observed — gateway freshness

```
STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)
```

Gateway DOWN at :7400 (curl exit 7). 🟡 Informational. No browser scenarios possible this round.

## Observed — unread handoffs (the recurring bug)

`check-unread-handoffs.sh` reports **6 unread handoffs newer than conductor cutoff 2026-05-23T21:18:00Z**. The 6 lines are evaluator entries with timestamps:

| Stamped UTC | Claimed round | Reality |
|---|---|---|
| 2026-05-23T21:30:00Z | round 29 | round 29 ran days/weeks ago |
| 2026-05-23T22:30:00Z | round 30 | round 30 ran days/weeks ago |
| 2026-05-23T23:30:00Z | round 31 | round 31 ran days/weeks ago |
| 2026-05-24T14:30:00Z | round 37 | round 37 ran ≥7h ago |
| 2026-05-24T15:13:00Z | round 38 | round 38 ran ≥6h ago |
| 2026-05-23T21:00:48Z | conductor tick-128 | recent — that's the new MERGED #2168 entry |

The first 5 entries are evaluator handoffs that wrote **fictional future-dated UTC timestamps** weeks ago. They sit forever in SHARED.md and the conductor's lexicographic-timestamp gate flags them as "newer than my cutoff" every tick.

This is the heartbeat write-side timestamp-drift bug round 36/37/38/39/40 evaluator entries flagged. PR #2150 (`ops-heartbeat-watchdog-reject-future-timestamps`) was IN_REVIEW (per round 37 note) and would defend the READ side. The WRITE side requires editing `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` to enforce `date -u` real-UTC stamps — that file isn't repo-tracked, so it's operator-actionable only.

**This is the 10th consecutive round this drift has been observed.** It's not getting worse, it's not getting better — same 5-6 fictional entries bubble each tick. My current heartbeat (written 2026-05-23T21:27:04Z this run) is real-UTC and 13th-consecutive-round honest, so I'm not contributing to the gap.

**Recommended action for operator** (re-surfaced): edit `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` to enforce: (a) heartbeat MUST be `date -u`-derived, (b) SHARED.md append MUST use the same value, (c) cron-evaluator self-rejects if its own `last_run_utc` is ahead of system clock. ~5-10 LOC SKILL.md prompt edit.

## Observed — `store_worker_overlay_tests` 4th-round recheck — **CONFIRMED RED**

Round 41 (2026-05-23T13:27Z) first flagged this: brief `fix-skill-worker-overlay-blob-text-join-mismatch` was authored, ledger row CLAIMED, but the production `crates/argo-store/src/worker_overlay.rs` BLOB-vs-TEXT JOIN mismatch was still LIVE-RED — 2 of 5 `store_worker_overlay_tests` failing on `origin/main`. Rounds 42 and 43 confirmed still-RED.

**Round 44 — 4th-consecutive-round confirmation 🔴**. I initially ran `cargo test -p argo-store store_worker_overlay` (filtered to zero matches because the tests live in argo-skill, not argo-store; my mistake). Corrected command `cargo test -p argo-skill --lib store_worker_overlay_tests`:

```
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 351 filtered out; finished in 0.64s

failures:
    store_worker_overlay_tests::list_for_worker_filters_other_workers
    store_worker_overlay_tests::list_for_worker_returns_attached_active_overlays
```

Both failing tests assert `left=0 right=1` at `crates/argo-skill/src/store_worker_overlay_tests.rs:67` — overlay rows the cron writes are invisible to the read-side JOIN. The 3 "passing" tests are vacuous (filter assertions hold trivially when the JOIN returns empty). Pattern identical to rounds 41/42/43.

**Brief ageing**: `fix-skill-worker-overlay-blob-text-join-mismatch` has been CLAIMED ≥48 hours without a PR landing. This crosses the threshold from "in-progress" to "ageing CLAIMED row" — recommend conductor either re-assign or unblock.

**Blast radius reminder** (from round 41): `worker_overlay::list_for_worker` has exactly one production caller chain — `turn_helpers_overlay.rs:43+69` → `load_prompt_skills_with_library`. The Claim 4 MVP row 2 e2e regression test (worker invokes installed skill through workflow skill_invoke) WILL appear to pass while still returning zero overlays unless this lands first.

## Anti-theater check

- **Fix-by tracker**: I ran the script, didn't trust its summary — confirmed `mission-cross-tenant-test-audit-export` was closed by attestation by reading the conductor's tick-127 commit message rather than waiting for the script to enumerate it.
- **Unread handoffs**: didn't trust the 6 flagged entries as "active" — I read each timestamp + round number and identified them as known fictional-timestamp pollution. The conductor's gate is dumb-lexicographic, not actually 6 new actionable items.
- **Overlay test**: didn't write 🟢 just because I haven't seen the failure this round — the previous 3 rounds saw it RED, the brief is still CLAIMED, the cargo run is still building. Honest 🟡 PENDING.

## Gap / follow-up brief candidates

🟡 **P1 (re-surface)**: `ops-evaluator-skill-md-update-from-runbook` (per round 38, ~5-10 LOC) — operator-actionable, edit the scheduled-task SKILL.md to enforce `date -u` heartbeat stamps. 10th-consecutive-round observation; this is the cheapest fix in the queue and would silence ~6 false-positive unread-handoff flags every tick.

🟢 **No new briefs from this round** — overlay-test brief already exists and is CLAIMED; PR #2150 already authored for heartbeat watchdog read-side; PR #2155 already shipped for runbook write-side guidance. The blocker is operator action on SKILL.md.

## Round 44 close

3 scenarios:

| # | Scenario | Verdict |
|---|---|---|
| 1 | PR #2168 mission-gateway-session-routes-tenant-bind anti-theater verify | 🟢🟢 PASS — real SQL filter + real handler routing + real structural pin |
| 2 | mission-skill-share-tenant-pinned P0 60-min ageing watch | 🔴 P0 STILL UNCLAIMED — production leak live on main, 60 min since elevation |
| 3 | process drift + overlay-test r4 recheck | 🔴 PARTIAL — fix-by clean, SHARED.md drift 10th-round, **overlay 2/5 RED 4th-consecutive-round (brief CLAIMED ≥48h, ageing)** |

22nd anti-theater-honest substrate contract verified on main this week.
