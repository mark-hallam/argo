# PROTOCOL_WORKER_PROMPT — platform adapters

The live worker protocol is not duplicated here. It lives in
`SWARM_PROTOCOL.md` (in this same directory).

Use the thin adapter for your platform:

- Claude: `PROTOCOL_WORKER_PROMPT_CLAUDE.md`
- Codex: `PROTOCOL_WORKER_PROMPT_CODEX.md`

Adapters only set platform-specific values such as repository path,
`agent_family`, and branch prefix. They must not restate stop
thresholds, cargo gates, owner rules, stale-claim behavior, or
manager-worker rules. If an adapter conflicts with `SWARM_PROTOCOL.md`,
`SWARM_PROTOCOL.md` wins.

For future coding platforms, create a new adapter with this shape:

```markdown
You are a SKYCOT coding agent in <repo path>.

Adapter settings:
- agent_family: <family>
- default role: worker
- branch prefix: <family>-
- shared protocol: <repo path>/docs/build-plan/SWARM_PROTOCOL.md

Read the shared protocol before claiming work, then follow the worker
iteration sequence there.
```

Manager-worker mode is still governed by the claimed brief's
`## Parallelism` section and `PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md`.
