# Your first worker

Argo's launch vertical is `gtm-engineer` — a marketplace seller that
researches a product, drafts a Facebook Marketplace listing, and
posts after operator approval.

This page takes you from a fresh install to a registered worker
ready for its first action.

## Confirm a vertical is available

```bash
argo vertical list
```

You should see `gtm-engineer`. If you don't, set
`ARGO_VERTICALS_DIR` to a directory containing the vertical
templates you want to load.

## Generate a vertical from a charter (Genesis)

If you want to operate a different role, use Genesis to scaffold one:

```bash
argo genesis interactive
```

You'll be walked through six prompts (role, name, industry, duties,
channels, spend cap). Genesis emits a charter, runs it through the
12-archetype classifier and the safety judge, and writes a vertical
template to `<workspace>/.skycot/verticals/<id>/`.

For an offline-only refinement pass, use:

```bash
argo genesis refine <vertical-id>
```

## Queue a worker job

```bash
argo work add \
  --vertical gtm-engineer \
  --prompt "Draft a Facebook Marketplace listing for a 2019 Yeti Tundra 65 cooler." \
  --json
```

This appends a `pending_jobs` row to the worker's memory. Nothing
runs yet.

## Run the worker once

```bash
argo work serve --vertical gtm-engineer --once
```

`--once` runs a single worker turn and exits. The worker reads its
queue, plans, emits actions, and stops. Without `--once`, the worker
serves continuously and re-checks its queue on a duty schedule.

## Confirm actions exist

```bash
argo worker actions list --state pending --vertical gtm-engineer --json
```

You should see one or more actions in the `pending` state, each
with a `worker_id`, `kind`, and `description`.

## Next

Move on to [Your first action](./first-action.md) to approve and
execute one.
