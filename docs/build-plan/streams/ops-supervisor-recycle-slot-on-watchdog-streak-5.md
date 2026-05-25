# ops-supervisor-recycle-slot-on-watchdog-streak-5 — watchdog self-recycles supervisor on sustained degraded streak

**Owner**: claude or codex. **Depends on**: `fix-worker-supervisor-respawn-after-rc-0` MERGED (PR #2339) — recycle is the safety net, not the primary fix.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-supervisor-recycle-slot-on-watchdog-streak-5`.

## Why this is P2

ω-r68 has reached RECURRENCE-5 because slot-1 has been DEAD since 01:02Z and the watchdog can only log alerts — it cannot recover the supervisor without operator intervention. Even after #2339 lands the primary fix, we want a defence-in-depth safety net so a future failure doesn't repeat the 21-hour outage.

## Scope

Add a streak-5 self-recycle escalation to `scripts/worker-supervisor-watchdog.sh`:
- When `streak >= 5`, attempt **once** (idempotent — record the last recycle timestamp in `~/.argo/watchdog-last-recycle.ts`) to call `launchctl kickstart -k gui/$(id -u)/com.skycot.worker-supervisor`.
- Only fire if `~/.argo/watchdog-last-recycle.ts` is absent OR older than 6 hours (so a stuck flap doesn't kickstart every minute).
- Always log the action to SHARED.md with `[supervisor-watchdog-recycle]` prefix + the streak count + the previous recycle timestamp.
- Operator can disable by setting `WATCHDOG_AUTO_RECYCLE=0` in their environment (default: `1`).

## Files you may touch

- `scripts/worker-supervisor-watchdog.sh` — add the streak-5 branch.

## Files you must NOT touch

- The LaunchAgent plist.
- `scripts/worker-supervisor.sh`.
- Anything under `crates/`.

## Acceptance criteria

- [ ] Streak <5 → no recycle attempt.
- [ ] Streak ≥5, no prior-recycle file → kickstart attempted + SHARED.md entry written + timestamp file created.
- [ ] Streak ≥5, prior recycle <6h ago → no second attempt; SHARED.md gets a "rate-limited" line instead.
- [ ] `WATCHDOG_AUTO_RECYCLE=0` → no recycle attempt regardless of streak.
- [ ] Manual test on a clean repo with `launchctl print` before/after shows the supervisor was actually recycled.

## Branch + PR

- Branch: `claude-ops-supervisor-recycle-slot-on-watchdog-streak-5` or `codex-…`.
- PR title: `ops(watchdog): self-recycle supervisor on streak ≥5 (ops-supervisor-recycle-slot-on-watchdog-streak-5)`.

## Notes

- ~50-80 LOC bash.
- Authorized under `--skip-clippy` per evaluator-pre-verified ops-* path.
