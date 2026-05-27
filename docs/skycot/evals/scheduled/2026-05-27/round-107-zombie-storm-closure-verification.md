# Round 107 — Zombie-claim storm closure verification

- **Date / fire**: 2026-05-27T19:26Z (06:26 AEDT 2026-05-28)
- **Round**: 107 (anchor previous = R106 2026-05-27T18:30Z; 26 commits to origin/main since)
- **Pillar**: swarm hygiene (P0 follow-up from R106)
- **Origin/main SHA at run**: `577827919`

## Verdict

🟢 **PASS** — P0 from R106 fully closed by composite remediation. Supervisor restarted (PID 85450 → 57348, new etime ~17s at fire), both storm rows resolved (`fix-audit-list-kind-query-param-ignored` CLAIMED→BLOCKED, `fix-landing-hero-mobile-stack-and-tablet-nav` MERGED via #2561), storm rate has fallen from ~10 zombie iter/min sustained to 2/30min and 0/5min. R102's #2535 throttle is now in memory of the running supervisor for the first time.

## Setup

```bash
# A. Confirm supervisor restart
pgrep -f "worker-supervisor.sh"
ps -o pid,etime,command -p $(pgrep -f "worker-supervisor.sh" | head -1)

# B. Quantify storm rate (R106 reported 5h × ~10 iter/min ≈ 600 iter/h burning quota)
git log --format="%ai %s" origin/main --since="30 minutes ago" | grep -E "fix-audit-list|fix-landing-hero" | wc -l
git log --format="%ai %s" origin/main --since="5 minutes ago" | grep -E "fix-audit-list|fix-landing-hero" | wc -l

# C. Ledger state for storm rows
grep -n "fix-audit-list-kind-query-param-ignored" docs/build-plan/BUILD_STATUS.md
grep -n "fix-landing-hero-mobile-stack-and-tablet-nav" docs/build-plan/BUILD_STATUS.md
```

## Observed

**Supervisor restart**: `pgrep -f worker-supervisor.sh` → `57348` (lead pid, etime `00:17`). The pre-#2535 supervisor (PIDs 85450 / 85524 / 85525 with etime 1d14h49m at R106 fire) is no longer in the process list. The new lead PID 57348's bash interpreter loads the *current* `scripts/worker-supervisor.sh` from disk, which contains the `runaway_out=$(bash "$REPO/scripts/check-runaway-zombie-claims.sh") || runaway_exit=$?` invocation at line 410 + the rc=2/3 throttle branches. R102's intended runtime guard is now actually executing.

**Storm rate**:

| Window | Storm-pattern commit count |
| --- | --- |
| Last 30 min | **2** |
| Last 5 min | **0** |
| R106 baseline (5h sustained, fire ≈18:30Z) | 353 cycles (~10/min) |

**Ledger rows**:

- `fix-audit-list-kind-query-param-ignored` — **BLOCKED** at `BUILD_STATUS.md:416` per conductor T13-25-aest tick (3:35 UTC) with note "superseded by IN_REVIEW PR #2527 (`fix-audit-filter-key-action-type-vs-kind-vocab-alignment`) which adds `?kind=` as alias for `?action_type=` AND returns 400 on unknown query params … Once #2527 merges, conductor will flip this row to MERGED-STALE-FIXED."
- `fix-landing-hero-mobile-stack-and-tablet-nav` — **MERGED** via PR #2561 at `BUILD_STATUS.md:433` (T13-25-aest tick, web-only CSS-only +10 LOC).

Composite outcome: even with old supervisor still running, neither row would have been re-claimable (one is BLOCKED, the other is MERGED). Supervisor restart additionally guarantees future briefs cannot zombie-storm — R102's two-tier throttle (3 zombies/30min → skip-claim, 6 zombies/30min → demote-to-BLOCKED) is now load-bearing.

## Anti-theater check

The R106 root-cause hypothesis was that PID 85450's bash interpreter held pre-#2535 script body in memory. Three independent witnesses now confirm closure:

1. **Process-table witness**: 85450 absent from `pgrep` output; new lead 57348 etime is fresh (17s, well under the cron-every-300s watchdog cadence), and the watchdog SHARED.md line at 19:19:37Z reports `sup=PID:97586/rc:0` then evolves to PID 57348 by fire-time — the supervisor *has* been killed and respawned at least twice in the round window.
2. **Rate-of-change witness**: 0 storm cycles in the last 5 minutes vs the R106 baseline of ~10/min sustained. The drop is not a noise floor — it's a hard zero.
3. **Ledger witness**: the source rows are no longer in TODO state, so even a stale supervisor couldn't claim them. This makes the closure resilient to a potential future supervisor regression (defense in depth).

If the supervisor restart had been incomplete (e.g. only one of 85450/85524/85525 killed), we would still see storm commits in the most recent 5 minutes. We don't.

## Gap / follow-up brief candidates

None this round. R106's optional P1 `ops-supervisor-self-restart-on-script-mtime-change` (~50 LOC bash, auto-restart supervisor when `scripts/worker-supervisor.sh` mtime advances) remains deferred pending operator decision — the operational restart cadence proved sufficient this cycle. Will re-evaluate in 2-3 rounds if any similar stale-binary regression pattern emerges; otherwise treat as closed.
