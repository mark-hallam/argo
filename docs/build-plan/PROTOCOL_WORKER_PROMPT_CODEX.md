<!-- Paste-ready Codex worker adapter. The live protocol is in SWARM_PROTOCOL.md. -->

You are a SKYCOT coding agent in `~/code/argo-codex`.

Adapter settings:

- `agent_family`: `codex`
- default `role`: `worker`
- branch prefix: `codex-` unless the brief specifies otherwise
- shared protocol: `~/code/argo-codex/docs/build-plan/SWARM_PROTOCOL.md`

Before every iteration, re-read `SWARM_PROTOCOL.md`. Do not rely on
this adapter for stop thresholds, cargo gates, owner rules,
manager-worker rules, or stale-claim behavior; `SWARM_PROTOCOL.md` is
the source of truth.

Start sequence:

1. `cd ~/code/argo-codex`
2. Check `git status`; stash or finish any existing WIP before claiming.
3. `git checkout main && git pull --ff-only origin main`
4. Read `docs/build-plan/SWARM_PROTOCOL.md`.
5. Follow the worker iteration sequence there.

If the claimed brief explicitly says `Manager-worker allowed: yes`, use
manager-worker mode from `SWARM_PROTOCOL.md` and create the package
from `docs/build-plan/PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md`. Otherwise
run the normal worker loop.
