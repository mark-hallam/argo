# Post-reboot swarm startup runbook

**Audience**: operator (Mark Hallam).
**When to use**: after a Mac reboot, after closing all Claude Code tabs, or any time you find the swarm partially or fully down.
**Estimated time**: ~5 minutes if everything cooperates.

## What does and does NOT survive a reboot

| Component | Survives reboot? | Notes |
|---|---|---|
| **macOS scheduled tasks** (auditor 3×/day, marketing weekday-morning, personal-site weekday-morning) | ✅ YES | These are real OS-level scheduled tasks. They fire on schedule without any action from you. |
| **Live gateway** at `:7400` | ❌ NO | Bound to a single `argo gateway start` process. Manual restart required for live SPA / E2E testing. |
| **Worker supervisor** (`scripts/worker-supervisor.sh`) + its 4 worker slots | ❌ NO | Launched via `nohup`; survives terminal close but **not** reboot. |
| **Conductor cron** (`3,23,43 * * * *`) | ❌ NO | Session-only — lives inside the Claude Code session. Dies when you close Claude Code or reboot. |
| **`~/.argo/env.local`** (env vars: `SKYCOT_DB_KEY_BASE64`, `ANTHROPIC_API_KEY`) | ✅ YES | It's a file on disk. Auto-sourced by `make restart-live-gateway` (per #2047). |
| **`~/.argo/sessions.db`** + worktrees in `/tmp/<brief-id>/` | ✅ YES (db) / ❌ NO (tmp) | Database persists. `/tmp` is wiped by macOS on reboot — any in-flight worker worktrees are lost (the brief returns to TODO when the conductor's stale-claim sweep catches it, ~2h later). |

## Recovery checklist — do these in order

### 1. Verify env (~30 seconds)

```bash
ls -la ~/.argo/env.local
# Should show: -rw------- (chmod 600), ~211 bytes, exports SKYCOT_DB_KEY_BASE64 + ANTHROPIC_API_KEY

cat ~/.argo/env.local | head -5
# Confirm both vars present. If empty or missing, see "env.local recovery" below.
```

If you need to recreate `env.local`, run `make first-run-setup` from the repo (shipped via #2047).

### 2. Start the live gateway — ONLY if you'll be testing the SPA or running E2E (~30 seconds)

```bash
cd ~/code/argo
make restart-live-gateway
```

This sources `~/.argo/env.local`, kills any stale gateway PID, rebuilds if needed, starts a fresh `argo gateway start`, polls `:7400/readyz` until ready, and writes the new PID file.

If you'll only be doing code work (not browser/SPA testing), **skip this**. The gateway is only needed for live HTTP traffic.

### 3. Launch the worker supervisor (~5 seconds)

```bash
cd ~/code/argo
nohup bash scripts/worker-supervisor.sh 4 > /tmp/supervisor.log 2>&1 &
disown
```

This spawns 4 supervised worker slots. Each slot auto-respawns when its worker exits (spin-loop guard, signal trap, etc.) — `scripts/worker-supervisor.sh` is the supervisor (shipped 2026-05-23).

**Verify it's running:**

```bash
tail -10 /tmp/supervisor.log
# Should show: "worker-supervisor.sh — keeping 4 workers alive" + "slot-N: launching worker → /tmp/claude-worker-supervised-slotN-*.log"

pgrep -fl worker-supervisor.sh
# Should show: ONE supervisor process (the parent) + 4 slot children.
```

**If it refuses to start** with "supervisor already running", a previous supervisor instance crashed without releasing its lock:

```bash
rm /tmp/argo-worker-supervisor.pid
# Then re-run the launch command above.
```

### 4. Open Claude Code in the repo (~10 seconds)

```bash
cd ~/code/argo && claude
```

Or open via the GUI. This is your interactive conductor session.

### 5. Re-arm the conductor cron in the new Claude Code session (~5 seconds)

In the freshly-opened Claude Code session, paste this prompt to restart the autonomous 20-minute tick loop:

```
Run one conductor tick per docs/build-plan/PROTOCOL_LOOP_PROMPT.md. Each tick: (0) check queue depth — claude_todo / codex_todo, fire Step 8 replenishment if ≤25; (1-4) sync main, recent commits, open PRs, ledger head; (4.5) check docs/skycot/audit/scheduled/LATEST.md for fresh P0s ≤12h old; (4.5b) bash scripts/check-unread-handoffs.sh — if exit 1, action each unread SHARED.md entry before declaring complete; (4.6) refresh docs/skycot/agents/heartbeats/conductor.json + append to docs/skycot/agents/SHARED.md; (4.7) bash scripts/check-gateway-freshness.sh — surface drift in tick summary; (5) for CLEAN PRs run Tier-1 review then admin-merge if good — flip BUILD_STATUS.md row to MERGED, push status commit; for DIRTY PRs post fix-request comments with rebase recipe; (5c) demote stale IN_REVIEW rows where PR has been DIRTY >2h since last fix-request; (7) BLOCKED triage — re-evaluate any rows whose deps just merged; (8) queue replenishment — author 3-5 new briefs if claude_todo or codex_todo ≤25 (idea sources in priority order: MISSION_BACKLOG[NEW] → PRODUCT_ROADMAP tracks → recent PR-body deferrals → git log -100); (9) brief tick summary report. Also during each tick: scan for zombie workers (claude in SN state + log silent >15min + last event was result:success — kill the chain, demote the orphan claim with salvage note). Be safe: never force-push main, never close PRs without operator approval, run pre-merge gate before admin-merge, use && chains so ledger flips only push if merges succeed. End each tick with a concise summary (under 200 words).
```

This re-creates cron job `d37c53a3` (or a fresh ID) and starts the conductor's autonomous 20-minute tick loop.

### 6. Verify the swarm is healthy (~30 seconds)

In the Claude Code session, ask:

```
worker status
```

Expected output:
- Supervisor + 4 supervised slots alive (PIDs visible)
- Pulse files in `/tmp/argo-worker-pulse/` updating within the last 30 seconds
- Worker logs in `/tmp/claude-worker-supervised-slotN-*.log` showing recent activity
- Cron `d37c53a3` listed via `CronList`
- `bash scripts/check-unread-handoffs.sh` returns `✅ No unread handoffs` (exit 0)

If any of these are missing, see the troubleshooting section below.

## Quick reference card

Pin this for fast recovery:

```bash
# === SKYCOT post-reboot ===
# 1. Verify env
cat ~/.argo/env.local | head -3

# 2. Start gateway (optional, only for live testing)
cd ~/code/argo && make restart-live-gateway

# 3. Launch worker supervisor
nohup bash ~/code/argo/scripts/worker-supervisor.sh 4 > /tmp/supervisor.log 2>&1 &
disown

# 4. Open Claude Code
cd ~/code/argo && claude

# 5. In Claude Code, paste the conductor cron prompt
#    (see step 5 above for the full prompt)

# 6. Verify
ps aux | grep -E "worker-supervisor|claude-worker-loop" | grep -v grep
bash ~/code/argo/scripts/check-unread-handoffs.sh
```

## Troubleshooting

### Gateway won't start

- `make restart-live-gateway` should self-diagnose. Common failure: `~/.argo/env.local` missing or invalid → run `make first-run-setup`.
- If you see "Keychain password" popup: click **"Always Allow"** to grant the SQLCipher key entry permanent access. This stops the popup recurring.

### Supervisor refuses to start

- Check `/tmp/argo-worker-supervisor.pid` — if the file exists but the named PID is dead, the lock is stale:
  ```bash
  rm /tmp/argo-worker-supervisor.pid
  ```
  then re-run the launch.
- If `ANTHROPIC_API_KEY` is set in your shell, the supervisor refuses to start (would propagate to workers and trigger their hard guard). Comment it out in `~/.zshenv` and open a fresh terminal.

### No workers pulsing

- `ls -la /tmp/argo-worker-pulse/` — if empty, the supervisor's slots haven't claimed briefs yet (normal for first ~30s after launch).
- If still empty after 2 minutes: `tail -50 /tmp/claude-worker-supervised-slot1-*.log` to see what slot-1 is doing. Common cause: ledger is empty (no TODO briefs to claim) — the conductor's Step 8 replenishment will fix that on the next tick.

### Conductor cron not firing

- Inside Claude Code, ask: `CronList` (or "show me current crons"). If `d37c53a3` (or similar) is missing, paste the conductor cron prompt again (step 5).
- Note: Claude Code crons are session-only. If you close the Claude Code tab, the cron dies. Reopen + re-arm.

### Scheduled-tasks not firing (auditor, marketing, personal-site)

These are OS-level scheduled tasks and should auto-fire on schedule. Verify with:

```bash
# In any Claude Code session
mcp__scheduled-tasks__list_scheduled_tasks
```

Should show 3 enabled tasks with `nextRunAt` in the future. If any are disabled or missing, the operator can re-create them via `mcp__scheduled-tasks__create_scheduled_task` (specifics in `docs/skycot/agents/AGENTS_OVERVIEW.md`).

### `~/.argo/env.local` recovery

If the file is missing or corrupted:

```bash
cd ~/code/argo
make first-run-setup
# This regenerates env.local with a fresh SKYCOT_DB_KEY_BASE64 + prompts for your ANTHROPIC_API_KEY.
```

The DB key in `env.local` MUST match what's already in the SQLCipher database. If you regenerate it from scratch, the existing `~/.argo/sessions.db` will refuse to open (wrong key). In that case, you have two options:

1. **Restore from Time Machine backup** — the key + db must come from the same point in time.
2. **Accept data loss** — `rm ~/.argo/sessions.db` and let the gateway create a fresh one (you lose all audit history + worker state).

Pick (1) if at all possible. The decryption key is THE load-bearing secret in the SKYCOT data layer.

## What's not in this runbook

- **launchd / OS-level persistence for the supervisor + conductor** — deferred (operator decided not to bring it that far on 2026-05-23). When ready, the next step is wrapping `worker-supervisor.sh` in a `~/Library/LaunchAgents/com.skycot.worker-supervisor.plist` and the conductor cron in `mcp__scheduled-tasks__create_scheduled_task` for OS-level reboot survival.
- **Evaluator on persistent cron** — currently ad-hoc / `/rerun`-triggered. Adding it would mirror the auditor's pattern.
- **Multi-machine swarm** — if you spin up workers on a second Mac, that's a separate runbook (each machine runs its own supervisor; the ledger on `origin/main` is the cross-machine source of truth).

---

**Last updated**: 2026-05-23 (tick-75, after `scripts/worker-supervisor.sh` shipped).
**Companion docs**:
- `docs/skycot/operator/gateway-restart-policy.md` — gateway restart semantics
- `docs/skycot/agents/AGENTS_OVERVIEW.md` — agent inventory + launch prompts
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — the conductor cron prompt (source of truth)
