# Initialise a workspace

A workspace is the folder where Argo stores its local configuration,
worker memory, credentials, audit data, and action output.

```bash
mkdir maya-demo
cd maya-demo
argo workspace init
```

Expected output:

```text
workspace initialized at /Users/you/maya-demo
  + argo.toml
  + skills/, personas/, cron/, rubrics/
  + MEMORY.md, USER.md, SOUL.md, AGENTS.md
  + db row: workspace 019...
```

Argo creates:

- `argo.toml` for workspace settings.
- `.argo/` and `.skycot/` for local runtime state.
- `skills/`, `personas/`, `cron/`, and `rubrics/` for worker assets.
- `MEMORY.md`, `USER.md`, `SOUL.md`, and `AGENTS.md` for operator-facing context.

Check the workspace:

```bash
argo workspace status
```

Expected output:

```text
workspace: maya-demo
db: ok
costs: 0 rows
```

If you see `no workspace found`, make sure the terminal is still in
the folder where you ran `argo workspace init`.

Run the broader health check:

```bash
argo doctor
```

Expected output includes:

```text
doctor: workspace ok
doctor: provider credentials not configured
doctor: channels not configured
```

The credential and channel warnings are expected until the next step.

## Next

Choose the worker template in [Pick a vertical](./pick-vertical.md).
