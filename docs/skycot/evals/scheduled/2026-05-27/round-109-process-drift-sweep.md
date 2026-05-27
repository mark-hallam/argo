# Round 109 — Process drift sweep

- **Date**: 2026-05-27 (UTC) / 2026-05-28 AEST
- **Round**: 109
- **Pillar / Claim**: Process drift (gateway freshness, findings-only budget,
  storm closure persistence, unread handoffs)
- **Verdict**: 🟡 PARTIAL · severity tier P3 (no operator action this round,
  but gateway-restart carry is one round from the brief-authoring threshold)

## Setup

Four drift probes:

1. `bash scripts/check-gateway-freshness.sh` — gateway schema vs codebase
2. `python3 scripts/find-stale-claim-drift.py --quiet` — ledger drift sweep
3. `bash scripts/check-findings-only-fix-by.sh` — findings-only fix-by tally
4. Supervisor PID + commit-cadence anti-storm reading vs R106 baseline
5. Unread-handoff sweep

## Observed

### 1. Gateway freshness

```
$ curl -sf http://127.0.0.1:7400/health
{"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":68590}

$ bash scripts/check-gateway-freshness.sh
gateway_schema=v83 codebase_latest=v84 drift=1
STATUS: FRESH  (drift <= 2)
```

- Gateway uptime **68590s ≈ 19h03m** (continuous since ~2026-05-27T04:24Z,
  i.e. before the R106 storm and well before the R107 supervisor restart).
- Schema **v83 vs codebase v84**, drift = 1 — still **FRESH** (ceiling = 2).
- `make restart-live-gateway` carry now at **10 rounds** (R100 → R109). R108's
  S3 set the brief-authoring threshold at "12+ rounds R110/R111 surfaces as
  operator-action headline". One round from threshold.

### 2. Ledger drift

```
$ python3 scripts/find-stale-claim-drift.py --quiet
Found 0 drift rows (none)
```

Zero rows. R107's storm-source row (`fix-audit-list-kind-query-param-ignored`)
was demoted CLAIMED→BLOCKED by conductor T13-25-aest after PR #2527 covered
the underlying fix; the other R106 storm row (`fix-landing-hero-mobile-stack-and-tablet-nav`)
MERGED via #2561.

### 3. Findings-only budget

```
$ bash scripts/check-findings-only-fix-by.sh
🟢 eval-cron-duty-fires-agent-runner-end-to-end       within budget (2026-05-30, 3 days remaining)
🟢 eval-workflow-execution-runs-real-step             within budget (2026-05-30, 3 days remaining)
🟢 mission-hire-audit-payload-shape-validation        within budget (2026-05-30, 3 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test      within budget (2026-05-30, 3 days remaining)

Tally: 4 within-budget · 0 due-today · 0 overdue · 0 informational · 4 total.
```

All four 3 days from fix-by. No action this round; will surface as P2/P3
candidates from R114 onward if not closed.

### 4. Anti-storm reading (R106 storm closure persistence)

R106 fire (2026-05-27T18:30Z) recorded ~10 claim/release commits per minute
on `fix-audit-list-kind-query-param-ignored` + `fix-landing-hero-mobile-stack`.
R107 closed the storm (PID 85450 restart → PID 97586 lead). R108 verified
persistence at the 1h post-restart mark.

This round (R109, fired 2026-05-27T23:27Z):

- **Lead supervisor PID 97586**, `etime` **04:36:32** (≈ 4h36m). Process tree
  reads cleanly:

  ```
  PPID=1  PID=97586  PGID=97586  STAT=S  bash worker-supervisor.sh 2
    └ PID=97610 (slot-1 loop), PID=97611 (slot-2 loop) — both children of 97586
        └ PID=57348 (slot-1 worker iter, etime 04:00:36)
        └ PID=74632 (slot-2 worker iter, etime 03:51:52)
  ```

  No orphan supervisors; the R106 PID 85450 family is **GONE** (was visible
  at R106 fire; absent for 4h+).
- **Commit cadence** sampled at fire:
  - last 5 min: 4 claim/release commits (mixed briefs)
  - last 30 min: 4 claim/release commits
  - vs R106 storm baseline of ~10/min sustained: storm closed
    **3-orders-of-magnitude** below R106 amplitude.

The R102 supervisor-throttle code at
`scripts/worker-supervisor.sh:407-428` is structurally what is loaded into
PID 97586's bash interpreter, so a re-storm on the same brief cannot recur
even if a row were to be re-promoted; PR #2535 shipped on 2026-05-27T15:09Z
and PID 97586 started 2026-05-27T18:50Z (well after).

### 5. Unread handoffs

```
$ bash scripts/check-unread-handoffs.sh

ghost-stamped conductor entries detected (2)
  - 2026-05-27T08:30:00Z [conductor] tick T08-30 …  (nearest commit 2924s away — 11th carry)
  - 2026-05-28T03:35:00Z [conductor] tick T13-25-aest scheduled …  (nearest commit 22980s away — 1st carry)

UNREAD HANDOFFS (2) newer than conductor cutoff 2026-05-27T21:11:00Z:
  - 2026-05-27T21:20:44Z [supervisor-watchdog] sup=PID:97586 slots=2/2 s1=R5-killed …
  - 2026-05-27T21:20:44Z [supervisor-watchdog] 🔪 R5-KILL claude pid=45738 slot=1 …

cutoff: 2026-05-27T21:11:00Z
ACK PATH: All unread handoffs are already-landed, operator-only, or informational.
```

- Both unread lines are **informational** — supervisor-watchdog R5-killed
  a post-completion hang on slot-1 (worker emitted the success summary then
  the descendant bash blocked in an `until grep` poll). No remediation
  needed; the supervisor's R5 path already handled it.
- The 11-round-carry ghost stamp (T08-30) crossed the pilot threshold (10
  rounds) one round ago — but R108 already noted it as "below pilot threshold
  for brief authoring" and the operator has not flagged it as actionable.
  Leaving as **carry** rather than authoring a process-pipeline brief: the
  ghost-stamp regression-pin recommendation already exists at
  `streams/ops-conductor-ghost-stamped-tick-detection.md` (shipped per the
  T20-08 status note).

## Anti-theater check

Drift checks are themselves the anti-theater witnesses:

- `find-stale-claim-drift.py --quiet` returning 0 is the assertion. If it
  rewrote-itself-green, the storm rows R106 surfaced would not have been
  detectable — but R106's drift output named both rows (verified in R106
  scenario file).
- `check-findings-only-fix-by.sh` reads `docs/build-plan/streams/eval-*`
  + `mission-*` files for `Fix-by:` headers, not a synthetic list. Any
  brief that misses the format is flagged. Currently 4 found → 4 reported,
  the parsing isn't suppressing anything.
- Gateway-freshness probe `curl /health` is the actual production endpoint.
- Process-tree witness uses `ps -axj` not a heartbeat synopsis; the
  supervisor heartbeat could lie, the process table cannot.

## Gap / follow-up brief candidates

**No new brief authored this round.**

Two carries to flag in the next-round tracking:

- **R110 / R111 — operator-action headline trigger**: if `make
  restart-live-gateway` hits 12-round carry (R109 = 10 rounds) without
  schema convergence to v84 or a manual restart, R110's process-drift
  scenario should surface this as an operator action item *with* a
  recommendation for `ops-conductor-restart-live-gateway-on-schema-bump-merge`
  follow-up scope (the v84-MERGE detection brief at `streams/fa24cb5cb`
  shipped to detect the bump but does not auto-restart the live process).
  Not authored now — premature by one round; if R110 hits the threshold,
  authoring at R110 is in scope.

- **R110 rotation target — Claim 3** per the R108→R109 plan (R107 did Claim 5,
  R108 did Claim 2, R109 did Claim 6). Test:
  `crates/argo-runtime/tests/claim_3_live_genesis_to_run.rs`.

- **Ghost-stamp pilot threshold reconsideration**: the T08-30 ghost stamp is
  at 11-round carry. R108 noted "below pilot threshold". If carry crosses 15
  rounds without a conductor refresh, R112 / R113 should surface a process
  pipeline brief — `ops-conductor-ghost-stamped-tick-detection` shipped a
  detector but did not flip what conductor *does* when a ghost stamp is
  detected. Process-pipeline scope, ≤80 LOC bash, deferred.

## Cost

~6K tokens. All cheap shell probes, no cargo runs.

129th anti-theater verification this week.
