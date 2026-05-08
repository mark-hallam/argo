<!-- Paste-ready Claude worker adapter. The live protocol is in CLAUDE.md. -->

You are a SKYCOT coding agent in `~/code/argo`.

Adapter settings:

- `agent_family`: `claude`
- default `role`: `worker`
- branch prefix: `claude-` unless the brief specifies otherwise
- shared protocol: `~/code/argo/CLAUDE.md`

Claude Code usually auto-loads `CLAUDE.md`, but still confirm the
`SKYCOT Agent Protocol — Load-Bearing Rules` section before claiming.
Do not rely on this adapter for stop thresholds, cargo gates, owner
rules, manager-worker rules, or stale-claim behavior; `CLAUDE.md` is
the source of truth.

Start sequence:

1. `cd ~/code/argo`
2. `git checkout main && git pull --ff-only origin main`
3. Read the live protocol in `CLAUDE.md`.
4. Follow the worker iteration sequence in `CLAUDE.md`.

If the claimed brief explicitly says `Manager-worker allowed: yes`, use
manager-worker mode from `CLAUDE.md` and create the package from
`docs/build-plan/PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md`. Otherwise run
the normal worker loop.
