# fix-worker-quota-sleep-overshoot-probe-resume

**Owner**: claude or codex — **P1 RELIABILITY (swarm-critical — cost the swarm ~7h of build time on 2026-06-13).**
**Depends on**: none.
**Authored against**: origin/main `b833643e7` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-worker-quota-sleep-overshoot-probe-resume

## Problem (verified live 2026-06-13 ~05:20)

When a worker's `claude -p` run hits an Anthropic usage limit, the loop's
quota handler (`scripts/claude-worker-loop.sh` ~lines 780-870) parses ONE
reset timestamp from the error text and sleeps until it (+5 min buffer),
heartbeating every 5 min. On 2026-06-12 22:11 both slots hit the FIVE-HOUR
rolling window but parsed/displayed a reset of "8:40pm" THE NEXT DAY (the
weekly-cap reset the message also carries) — and armed a **~22-hour
sleep** for what was a ~3-hour outage. The conductor live-probed the same
CLI auth at 05:17 (`claude -p "say ok"` → OK in 11s, the operator-
authorized probe) while both slots still showed `💤 quota-sleep — 928min
remaining`. The conductor force-killed the loops to recover (SIGTERM was
absorbed — see item 3).

## Required change

1. **Probe-resume inside the sleep loop.** The heartbeat loop already
   wakes every 300s. Every Nth heartbeat (default every 6th ≈ 30 min, env
   `QUOTA_PROBE_INTERVAL_TICKS`), fire the operator-authorized 1-token
   probe (`claude -p` with a trivial prompt, 60s ceiling — the EXACT
   pattern memory `feedback_watchdog_quota_probe_authorized` locks). On
   success: log `✅ quota probe OK — resuming early`, break the sleep,
   `continue` the loop. On failure: keep sleeping. This makes the parsed
   reset time advisory, not load-bearing.
2. **Cap the parsed sleep.** Clamp `SLEEP_SECS` to a ceiling (default 2h,
   env `QUOTA_SLEEP_CAP_SECS`) — with probe-resume in place a short cap
   costs at most one failed probe per cap window, and a misparsed
   far-future reset can never strand a slot for a day.
3. **Fix signal handling during the sleep.** SIGTERM to the loop did NOT
   interrupt the quota-sleep (the conductor needed SIGKILL). Bash defers
   traps while waiting on a foreground child — run `sleep` in the
   background and `wait` on it (`sleep "$THIS_TICK" & wait $!`), so the
   trap fires immediately and the documented Ctrl-C/TERM release path
   actually works mid-sleep.
4. **Tests.** Bash assertions in the existing harness style: (a) a fake
   `claude` stub (PATH-injected) that fails twice then succeeds → the
   sleep loop exits early on the 3rd probe tick; (b) `SLEEP_SECS` larger
   than the cap is clamped; (c) a TERM mid-sleep exits within one tick.
   Keep ticks tiny via env so the suite runs in seconds.

## Verify

- New assertion script green; `shellcheck` clean on the touched script
  (no new findings vs main).
- Manual: arm a 60s fake quota-sleep with a succeeding stub probe and
  watch it resume on the first probe tick.

## May touch

- `scripts/claude-worker-loop.sh` (quota handler + heartbeat loop)
- new test/assertion script
