# Worker loop runbook

How to launch, monitor, and stop the stateless Claude worker loop.

The worker loop is a bash wrapper that drives an unattended Claude
Code worker by spawning a fresh `claude -p` headless session per
brief. Outer loop (state machine) is bash; inner (implementation
work) is Claude with a fresh context window every iteration. No
context accumulation, no compaction, no graceful-exit needed.

The conductor stays in the desktop app under `/loop 5m` — independent
of this loop. The conductor merges PRs that the loop ships.

## One-time setup

Set the shared cargo target dir so worker iterations reuse compiled
crates instead of rebuilding from scratch (~28 min cold → ~30s-2min
warm):

```bash
echo 'export CARGO_TARGET_DIR=/tmp/argo-shared-target' >> ~/.zshrc
source ~/.zshrc
```

Optional but recommended: install `sccache` for cross-tab compile
caching:

```bash
brew install sccache
echo 'export RUSTC_WRAPPER=sccache' >> ~/.zshrc
source ~/.zshrc
```

Verify prerequisites:

```bash
which claude gh awk          # all three must exist
gh auth status                # must show "Logged in to github.com"
ls ~/code/argo/scripts/claude-worker-loop.sh   # must exist
```

## Launch (one worker)

```bash
bash ~/code/argo/scripts/claude-worker-loop.sh
```

You'll see time-stamped iteration lines:

```
[06:42:11] Sync main...
[06:42:13] Claiming claude-worker-presence-tracking...
[06:42:14] Spawning fresh Claude session for claude-worker-presence-tracking...
... (Claude streams its work)
[06:51:33] Iteration done. Jittering 19s...
[06:51:52] Sync main...
... (next brief)
```

## Launch (multiple workers in parallel)

Open N terminal tabs, run the same command in each. They share
`BUILD_STATUS.md` as the lock — `claim-with-retry.sh` handles
race-loss correctly, so two workers won't ship the same brief.

```bash
# In each new terminal:
bash ~/code/argo/scripts/claude-worker-loop.sh
```

**Empirical guidance: start with one worker.** Per `PROTOCOL.md`,
4–6 is a "sweet spot" but observation on this machine shows
concurrent cargo+claude load + multiple worktrees can cause
integration smoke tests to hang on gateway boot (documented in
multiple worker self-exit notes). Single-worker iterations finish
faster than two competing under load. Add a second worker only after
the first is steady-state and your machine has CPU headroom.

## Stop

Graceful (lets the current iteration finish, then exits at the next
loop boundary):

```bash
touch /tmp/argo-stop
```

Immediate:

```
Ctrl-C
```

To re-enable the loop after a graceful stop, remove the sentinel:

```bash
rm /tmp/argo-stop
```

## Monitoring

While the loop runs, watch the swarm from another terminal:

```bash
# What's landing on main?
cd ~/code/argo
watch -n 30 'git log --oneline origin/main -5'

# Open PRs in flight?
watch -n 60 'gh pr list --state open --limit 20'

# Queue health?
cd ~/code/argo
grep -c "TODO" docs/build-plan/BUILD_STATUS.md
```

## Troubleshooting

**The loop sleeps and prints "No claimable TODO row":** queue is
drained or all TODOs have unmet dependencies. The conductor
replenishes when `codex_todo` or `claude_todo` drops below 8 (Step 8
of `PROTOCOL_LOOP_PROMPT.md`). Wait for the conductor's next tick.

**Claude session exits non-zero:** the loop logs and continues. The
brief stays CLAIMED on main; the conductor picks up stale-claim
sweeps in step 7 of its tick and either expands the may-touch list
or releases the row. Next loop iteration claims a different brief.

**`git pull --ff-only` fails inside the loop:** another tab pushed
during the script's iteration. The loop will retry on the next pass
because step 1 re-pulls. If it persists across iterations, your
local main has diverged — investigate manually.

**First brief is taking ~28 minutes:** cold cargo cache. Confirm
`CARGO_TARGET_DIR=/tmp/argo-shared-target` is exported (`echo
$CARGO_TARGET_DIR`). Subsequent briefs will be 30s-2min once the
shared cache warms.

**Brief BLOCKED with "premise stale" reason:** an older brief whose
file paths or assumptions don't match current main. The conductor
will rewrite or release at next tick. Loop continues.

**Loop exits with `Same brief failed 2× in a row`:** a brief's
ledger row is shaped in a way that `claim-with-retry.sh`'s python
regex rejects but the bash awk picks. Usually means the description
contains a markdown-quirk character the regex isn't expecting.
Inspect with `grep '<brief-id>' ~/code/argo/docs/build-plan/BUILD_STATUS.md`.
Fix the row (escape problem characters or relax the regex), then
relaunch the loop. The 2-strikes guard exists to prevent runaway
spin-loops at 1Hz (which previously hammered git fetches enough to
break worktree state).

**Integration smoke test hangs (gateway boot):** documented pattern
in worker self-exit notes — under heavy concurrent cargo load, the
gateway-boot listener can hang for hours. If the loop output shows
many repeated `🔧 TaskOutput {"task_id":"...","block":true,...}`
calls, the smoke test is hung. Ctrl-C the loop, release the stuck
CLAIM via the BUILD_STATUS row (flip CLAIMED → TODO), reduce
concurrent worker count, restart.

**Stuck CLAIM from a dead worker:** if a previous worker tab died
mid-iteration, its CLAIM row stays in the ledger. The conductor
sweeps these in step 7 of its tick, but you can manually flip
CLAIMED → TODO faster. Edit the row, commit + push to main with
message `status: release <brief-id> (stale CLAIM)`.

## What this loop does NOT do

- Does not merge PRs. Conductor's job.
- Does not author new briefs. Conductor's job (Step 8 queue
  replenishment).
- Does not edit `CLAUDE.md` or `SWARM_PROTOCOL.md`. Workers stay
  inside the brief's may-touch list.
- Does not pre-warm the cargo cache. The first iteration pays cold
  cost unless `CARGO_TARGET_DIR` is set + already warm from another
  tab.

## Related docs

- [`docs/build-plan/SWARM_PROTOCOL.md`](build-plan/SWARM_PROTOCOL.md) — the live operational rules.
- [`docs/build-plan/PROTOCOL.md`](build-plan/PROTOCOL.md) — architecture and anti-patterns.
- [`docs/build-plan/PROTOCOL_LOOP_PROMPT.md`](build-plan/PROTOCOL_LOOP_PROMPT.md) — conductor tick prompt.
- [`docs/build-plan/PROTOCOL_WORKER_PROMPT_CLAUDE.md`](build-plan/PROTOCOL_WORKER_PROMPT_CLAUDE.md) — worker prompt with CLI loop mode section.
- [`scripts/claude-worker-loop.sh`](../scripts/claude-worker-loop.sh) — the script itself (100 LOC, readable).
