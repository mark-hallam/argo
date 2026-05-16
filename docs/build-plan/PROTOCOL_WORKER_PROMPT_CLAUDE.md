<!-- Paste-ready Claude worker adapter. The live protocol is in SWARM_PROTOCOL.md. -->

You are a SKYCOT coding agent in `$ARGO_REPO_ROOT` (set per-machine; see
`docs/skycot/MULTI_MACHINE_OPS.md` and `scripts/bootstrap-machine.sh`).

Adapter settings:

- `agent_family`: `claude`
- default `role`: `worker`
- branch prefix: `claude-` unless the brief specifies otherwise
- shared protocol: `$ARGO_REPO_ROOT/docs/build-plan/SWARM_PROTOCOL.md`

Claude Code auto-loads `CLAUDE.md`, which points at `SWARM_PROTOCOL.md`.
Read the shared protocol before claiming. Do not rely on this adapter
for stop thresholds, cargo gates, owner rules, manager-worker rules, or
stale-claim behavior; `SWARM_PROTOCOL.md` is the source of truth.

Start sequence:

1. `cd "$ARGO_REPO_ROOT"`
2. `git checkout main && git pull --ff-only origin main`
3. Read `docs/build-plan/SWARM_PROTOCOL.md`.
4. Follow the worker iteration sequence there.

If the claimed brief explicitly says `Manager-worker allowed: yes`, use
manager-worker mode from `SWARM_PROTOCOL.md` and create the package
from `docs/build-plan/PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md`. Otherwise
run the normal worker loop.

## CLI loop mode (alternative to interactive desktop)

For unattended runs, launch the stateless Claude worker loop in any
terminal:

```bash
bash scripts/claude-worker-loop.sh
```

Optional env: `REPO=/path/to/repo`, `FAMILY=claude`. The wrapper holds
the swarm-protocol state machine in bash (pull, claim, jitter); each
iteration spawns a fresh `claude -p` session for one brief and exits.

CLI-loop workers are exempt from the graceful-exit protocol in
`SWARM_PROTOCOL.md` — every iteration is a fresh window, so context
never accumulates and there is no degraded-session mode to exit from.
Stop the loop by `touch /tmp/argo-stop` or Ctrl-C.

## Disk cleanup after each iteration (handled by the loop)

Each iteration creates a per-brief worktree at `/tmp/$BRIEF_ID` and a
cargo target dir at `/tmp/$BRIEF_ID-target` (often several GB). Without
explicit cleanup, /tmp fills up over a long-running loop (the swarm
typically runs 24–48hrs continuously during ship windows).

The cleanup is enforced at **two levels**:

1. **Worker-side (step 9 in the prompt that `claude -p` receives).**
   After PR push + ledger update OR the BLOCKED-exit path, the worker
   runs:

   ```bash
   cd $REPO
   git worktree remove --force /tmp/$BRIEF_ID 2>/dev/null || true
   rm -rf /tmp/$BRIEF_ID-target /tmp/argo-shared-target-$BRIEF_ID 2>/dev/null || true
   git worktree prune
   ```

2. **Outer-bash safety net.** If the worker skipped step 9 (quota
   pressure, context cap, network glitch mid-push, etc.), the bash
   wrapper sweeps the same paths defensively at the end of each
   iteration. Every 10 iterations it logs `/tmp` disk usage so the
   operator can spot leaks visually.

Hard rules (also embedded in the worker prompt):

- Always `cd $REPO` BEFORE `git worktree remove`. Git refuses if cwd
  is inside the target worktree.
- Use `git worktree remove`, NEVER `rm -rf` the worktree directly —
  raw rm-rf corrupts git's internal worktree state.
- NEVER touch `~/.cargo/registry/`. That's the cross-iteration cache;
  losing it costs many minutes per restart.
- If `git worktree remove` fails because of uncommitted changes, the
  PR probably didn't push cleanly. Don't force; investigate.

Codex workers (one-shot per session, not looped) have a parallel
mandate in `AGENTS.md` + `PROTOCOL_WORKER_PROMPT_CODEX.md`.
