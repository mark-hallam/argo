# Round 106 — P0 zombie-claim storm REGRESSION (supervisor process stale, pre-#2535)

**Date**: 2026-05-27 (UTC) / 2026-05-28 04:30 AEST
**Round**: 106
**Pillar / Claim**: Process — multi-agent swarm hygiene (covers all 6 claims indirectly: quota drain blocks worker progress on every track)

## Verdict

🔴 **FAIL — P0 (production-incident severity)** — runaway zombie-claim storm has been raging for >5 hours and is **still active at fire time**, despite #2535 (the throttle that was authored specifically to stop this storm class) merging ~3h ago.

## Setup

```
# Detect storm rate
git fetch origin main --quiet
git log --format="%ai %s" origin/main --since="1 hour ago" | wc -l
# 464 commits in the last 1h

git log --format="%ai %s" origin/main --since="1 hour ago" | grep -c "fix-audit-list-kind-query-param-ignored"
# 293

git log --format="%ai %s" origin/main --since="1 hour ago" | grep -c "fix-landing-hero-mobile-stack-and-tablet-nav"
# 36

# Total storm count since first zombie commit (2026-05-27 22:57:57 +1000 = 12:57:57Z)
git log --format="%ai %s" origin/main | grep -c "fix-audit-list-kind-query-param-ignored"
# 317
git log --format="%ai %s" origin/main | grep -c "fix-landing-hero-mobile-stack-and-tablet-nav"
# 36

# Inspect supervisor process state
ps aux | grep worker-supervisor | grep -v grep
# operator 85450 ... Tue01PM ... bash scripts/worker-supervisor.sh 2
# operator 85524 ... Tue01PM ... bash scripts/worker-supervisor.sh 2
# operator 85525 ... Tue01PM ... bash scripts/worker-supervisor.sh 2
ps -o etime= -p 85450
# 01-14:49:40   (1 day 14h 49m process uptime)

# Confirm throttle wiring is on disk but supervisor needs reload
grep -n "check-runaway-zombie-claims" scripts/worker-supervisor.sh
# 410:      runaway_out=$(bash "$REPO/scripts/check-runaway-zombie-claims.sh") || runaway_exit=$?

git log --oneline scripts/worker-supervisor.sh | head -2
# 22d769ddb ops: supervisor guard against runaway zombie-claim storms (#2535)
# 363bcecff swarm: workers + supervisor claim any TODO row regardless of owner column

ls -la scripts/worker-supervisor.sh
# -rwxr-xr-x@ 1 operator staff 20932 May 28 01:09 scripts/worker-supervisor.sh
```

## Observed

**Storm timeline**:
- **2026-05-27T12:57:57Z** — first zombie release commit (`fix-audit-list-kind-query-param-ignored`) hits origin/main.
- **2026-05-28T01:09:34Z (15:09Z UTC)** — #2535 merges (`scripts/check-runaway-zombie-claims.sh` + `scripts/demote-to-blocked.sh` + `scripts/worker-supervisor.sh:410` invocation).
- **2026-05-27T18:01:49Z** — latest zombie-release commit (most recent verified at fire time). **3h post-merge, storm STILL active.**

**Cumulative storm counts to origin/main**:
- `fix-audit-list-kind-query-param-ignored`: **317** zombie cycles
- `fix-landing-hero-mobile-stack-and-tablet-nav`: **36** zombie cycles
- Combined rate in last 1h: 329 commits = ~5.5/min

**Comparison to R102 baseline** (the storm that drove #2535 authoring):
- R102: 83 zombie cycles in 16 min = ~5.2/min on `fix-audit-list-kind-query-param-ignored`
- R106: ~5/min sustained for >5h = **>4× more total damage already, still ongoing**

**Ledger state**:
- Both storm rows still **TODO** in `docs/build-plan/BUILD_STATUS.md` (`fix-audit-list-kind-query-param-ignored` has hundreds of `claimed via claim-with-retry on mac-host at <timestamp>` clauses chained on a single row; `fix-landing-hero-mobile-stack-and-tablet-nav` similarly never demoted to BLOCKED).
- Per #2535's brief: 6 zombies in 30min should force demotion to BLOCKED via `scripts/demote-to-blocked.sh`. Threshold exceeded by ~50×; **demotion never fires.**

**Root cause** (dispositive evidence):

1. Supervisor processes (PIDs 85450/85524/85525) started **2026-05-26T03:38Z UTC** (1d 14h 49m process uptime at fire time, command stamp `Tue01PM` AEST).
2. #2535 (which added the throttle invocation at `scripts/worker-supervisor.sh:410`) merged **2026-05-28T01:09:34Z UTC**, ~46h after the supervisor started.
3. Bash supervisor interpreter loads its script body into memory at startup; long-running outer loops do **NOT** re-read the file. The in-memory copy is pre-#2535 (does not contain the `runaway_out=$(bash "$REPO/scripts/check-runaway-zombie-claims.sh")` invocation).
4. Therefore the throttle is shipped + on disk + executable but **never called** by the running supervisor.

## Anti-theater check

This is not "test rewrites itself green" — the **production failure mode is actively raging right now** while I write this report. Every minute that passes, ~5-6 more zombie-release commits land on origin/main, the ledger row grows by another 60+ chars of `claim-with-retry` noise, and the claude worker quota burns one more iteration. Cargo + git history are the ground truth, not a synthetic test.

The fix #2535 is **correct code** (verified by inspection of `scripts/check-runaway-zombie-claims.sh` thresholds: 3/30min skip, 6/30min block — both crossed by orders of magnitude in current storm). What is broken is the **deployment**: the running interpreter holds stale bytecode.

## Gap / follow-up

### IMMEDIATE — operator action required (no code change, no brief)

```bash
launchctl unload ~/Library/LaunchAgents/com.skycot.worker-supervisor.plist
launchctl load   ~/Library/LaunchAgents/com.skycot.worker-supervisor.plist
```

Or simply:

```bash
kill 85450 85524 85525   # LaunchAgent will auto-respawn supervisor with new code
```

Expected outcome on next post-restart iteration: throttle counts 317+36 zombies in last 30 min → far exceeds 6-block-threshold → both rows force-demoted to BLOCKED via `scripts/demote-to-blocked.sh` → workers no longer pick them up → storm halts.

### Optional P1 follow-up brief (operator-decision deferred)

**`ops-supervisor-self-restart-on-script-mtime-change`** — supervisor checks `stat -f %m scripts/worker-supervisor.sh` once per outer-loop iteration and exits if the file mtime is newer than the supervisor's own start time. LaunchAgent auto-respawns. ~50 LOC in `scripts/worker-supervisor.sh`. Closes the class of "merged-but-not-deployed" supervisor bugs.

**Brief NOT authored this round** — wanted to surface to operator first since:
- It adds complexity to a critical long-running daemon
- An alternative is operationalizing "restart supervisor after any merge that touches `scripts/worker-supervisor.sh` or `scripts/check-runaway-*` or `scripts/demote-to-blocked.sh`" via a conductor or post-merge hook
- Two plausible designs (self-monitor vs external hook); evaluator does not have evidence for picking one

### What this round does NOT recommend

- Do NOT manually edit the ledger to demote the rows — workers would just re-claim them on next iter. Demotion only sticks if supervisor refuses to honor new claims, which requires the restart.
- Do NOT force-push a revert of #2535 — the code is correct; only the running process is stale.
- Do NOT spawn workers to "rebase the storm briefs" — the briefs themselves may be fine; we don't know yet because nothing has actually attempted a fix (every iter so far has gone zombie before producing a PR).

## Severity rationale

P0 because:
1. **Quota burn** — at ~10 zombie iter/min, the claude worker pool is doing 600 iter/hour of pure waste. At any realistic per-iter token cost this is hundreds of thousands of tokens/day evaporated.
2. **Ledger pollution** — the `fix-audit-list-kind-query-param-ignored` BUILD_STATUS.md row is now ~30KB of `claim-with-retry` noise on a single row, making manual ledger inspection impractical.
3. **History pollution** — origin/main commit log has lost any signal-to-noise for the last 5+ hours (98%+ of commits are zombie churn).
4. **Process trust** — operator + future evaluators saw #2535 merge with R103 PRE-MERGE PASS + R104 explicit closure tracking, and conductor flipped the row to MERGED at T01-07-AEST. The system says the storm is fixed. The reality says otherwise.

## Source

This report. Round 105 closure of #2535 inner-loop was real (PR + brief landed cleanly); the deployment gap is what's broken.
