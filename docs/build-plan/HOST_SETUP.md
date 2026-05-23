# Host setup canary

Cheap insurance against the next host migration eating 40+ minutes
of swarm wall-clock and 86+ zombie commits while everyone figures
out why the supervisor is releasing every brief it claims.

If you only do one thing on a fresh host before launching the
supervisor: **run the 30-second canary at the bottom of this doc.**

## The problem this prevents

2026-05-23 mac-host doom-loop (tick-96 → tick-99b, ~40 minutes,
~91 commits in 20 min): the supervisor was launched from a shell
whose `PATH` did not include `~/.local/bin` (Claude Code's
standard install dir, populated by the auto-updater earlier the
same day). Every worker iteration:

1. `claim-with-retry.sh` succeeds → pushes `status: claim <brief>`
2. `claude --dangerously-skip-permissions ... | tee | jq` runs →
   bash dies on `claude: command not found` in ~5 seconds
3. Ship-check at `scripts/claude-worker-loop.sh:649` sees the row
   still `CLAIMED` → flips to `TODO`, pushes
   `status: release <brief> (zombie; no PR opened)`
4. Loop continues, picks the next `TODO`, repeats — ~25-30s per
   cycle, ~2 git pushes per cycle

86 zombie-status commits in one 20-min window across 8+ briefs
before the conductor demoted enough rows to `BLOCKED` to halt the
spread. Every per-iteration log file at
`~/.argo/worker-logs/*.ndjson` was exactly 93 bytes — the error
message and nothing else.

## The canary in the worker-loop

Both `scripts/claude-worker-loop.sh` (lines 55-89) and the
supervisor binary now refuse to start if `claude` is not on PATH.
On miss they:

1. Auto-repair the common case by exporting
   `$HOME/.local/bin:$PATH` and re-checking.
2. If still unresolvable, hard-fail with exit code `2`. This
   matches the `ANTHROPIC_API_KEY` guard so the supervisor (line
   104 of the same script) interprets it as **real config error
   — do NOT respawn this slot**, stopping the doom loop at the
   supervisor level.

The block is anchored by the source-code comment header `HARD
GUARD: refuse to run if 'claude' binary is not on PATH` so it
survives line-number drift.

## Why `~/.zshrc` (interactive) isn't enough

`~/.zshrc` is sourced only by **interactive** zsh instances. The
worker-loop's bash sub-shells, the supervisor's spawned `claude`
subprocesses, and any non-interactive zsh invocations skip it
entirely — they get the system default `PATH`, which on macOS is
`/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin` and does NOT
include `~/.local/bin`.

Export from `~/.zshenv` instead. `~/.zshenv` is sourced by every
zsh invocation, interactive or not:

```bash
# ~/.zshenv
export PATH="$HOME/.local/bin:$PATH"
```

For bash users the equivalent is `~/.profile` (login shells) plus
`~/.bashrc` (interactive non-login). Non-interactive bash sources
whatever `BASH_ENV` points at — confirm with:

```bash
bash -c 'echo $PATH'
```

## 30-second canary before launching the supervisor

Run this from a fresh terminal — NOT the terminal you've been
sourcing things from interactively. If both checks return a
resolvable path (not "command not found"), your host is safe to
launch the supervisor:

```bash
bash -c "which claude" && bash -c "echo \$PATH"
```

Expected output (paths will vary by user):

```
~/.local/bin/claude
~/.local/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
```

If `which claude` fails:

1. Run `which claude` from your interactive shell. If THAT works,
   the binary is installed but only your `~/.zshrc` knows where —
   move the PATH export to `~/.zshenv` (see above) and open a
   fresh terminal.
2. If your interactive shell also can't find it, re-run the
   Claude Code installer or check that `$HOME/.local/bin/claude`
   exists on disk.

## Related artifacts

- **Fix commit**: `7e9b1426a` ("worker-loop: fail-fast if `claude`
  binary is not on PATH") — adds the canary block to
  `claude-worker-loop.sh` + the supervisor.
- **Operator amendment**: `~/.zshenv` `PATH` export on the
  operator's machine (post-incident).
- **Conductor sweep**: the auto-release of zombie claims by
  `scripts/check-zombie-workers.sh` (from `claude-worker-loop-zombie-detection`,
  #2038) is what eventually halts the doom loop today — but only
  after some zombie commits have already landed. The canary is
  the upstream fix that stops the loop before it starts.
- **Related onboarding docs**: `docs/build-plan/RESUME.md`
  (fresh-agent primer), `docs/build-plan/CONTRIBUTING_AGENT.md`
  (worker rules), `docs/build-plan/SWARM_PROTOCOL.md` (HARD RULES).

Author: `ops-host-setup-runbook-path-canary` (P2 audit follow-up,
2026-05-23T09-40 §4 P2 #5).
