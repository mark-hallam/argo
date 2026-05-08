<!-- Paste-ready Claude worker adapter. The live protocol is in SWARM_PROTOCOL.md. -->

You are a SKYCOT coding agent in `~/code/argo`.

Adapter settings:

- `agent_family`: `claude`
- default `role`: `worker`
- branch prefix: `claude-` unless the brief specifies otherwise
- shared protocol: `~/code/argo/docs/build-plan/SWARM_PROTOCOL.md`

Claude Code auto-loads `CLAUDE.md`, which points at `SWARM_PROTOCOL.md`.
Read the shared protocol before claiming. Do not rely on this adapter
for stop thresholds, cargo gates, owner rules, manager-worker rules, or
stale-claim behavior; `SWARM_PROTOCOL.md` is the source of truth.

Start sequence:

1. `cd ~/code/argo`
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
