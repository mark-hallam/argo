# E2B idle-sandbox reaper

Operator runbook for `argo sandbox e2b list` /
`argo sandbox e2b reap-idle` and the periodic cron job that wraps
them (Stream `mission-e2b-reap-idle`).

## What it does

The E2B computer-use sandbox is a Firecracker microVM that Argo
opens via the [E2B managed service](https://e2b.dev). Healthy CU
sessions close their sandbox via the executor's `Drop` path at
shutdown. If the worker crashes mid-run, the sandbox leaks — E2B
keeps billing until the template's `pause_timeout` elapses
(typically 5–15 min).

The reaper is the **out-of-band** cleanup path. It lists every
sandbox the configured E2B API key owns, filters those idle longer
than `--max-age`, and closes them explicitly. The in-band CU
cleanup is unchanged — the reaper only matters for leaks.

## ⚠️ Tenancy boundary

**The E2B API key is the tenancy boundary.** Reaping closes ALL
idle sandboxes under that key, **including sandboxes from other
Argo workspaces sharing the same key, and sandboxes opened by
other E2B-using applications**. There is no workspace-level
filter (E2B doesn't tag sandboxes with workspace identity).

If you run multiple Argo workspaces or other E2B apps under one
key, **dry-run first**:

```sh
argo sandbox e2b reap-idle --max-age 600 --dry-run
```

Inspect the `would_reap` ids before running without `--dry-run`.

## Surfaces

### One-shot list

```sh
ARGO_E2B_API_KEY=<key> argo sandbox e2b list
```

Emits one JSON object per line (one row per sandbox). Pipe to
`jq` for filtering / aggregation.

### One-shot reap

```sh
ARGO_E2B_API_KEY=<key> argo sandbox e2b reap-idle --max-age 600
```

Closes every sandbox idle > 600 seconds (10 minutes — the default).
Emits a single JSON summary:

```json
{"scanned":4,"reaped":["sb-abc","sb-def"],"errors":[]}
```

Reaping continues past per-sandbox close errors; the `errors`
array carries `[id, message]` pairs for any sandbox whose close
failed.

### Dry-run

```sh
argo sandbox e2b reap-idle --max-age 600 --dry-run
```

Lists candidate ids without closing them. Always run before the
first real reap on a shared key.

### Periodic cron

The reaper is also registered as a cron job in the runtime, ticking
every 15 minutes with `max_idle_secs=600`. **Gated by
`ARGO_E2B_API_KEY` presence** — operators without E2B configured
see no logs and no errors (the cron path uses `tracing::trace!`
for the no-op skip).

When the workspace's cron scaffolding
(`mission-cron-tick-maintenance-jobs`) lands, this hook moves to
`argo cron tick`; until then it lives in
`argo_executor_computer_use::e2b_reap_cron::run_e2b_reap_cycle`
and is invoked by the gateway's housekeeping loop.

## When to run manually

- **Post-crash recovery.** A worker process was killed (OOM, sigkill,
  panic) before its `Drop` path could close its sandbox. The cron
  picks this up within 25 min; manual run closes it sooner.
- **Operator forgot to close a long session.** The dev opened a
  sandbox interactively (e.g. via `argo cu replay`), got distracted,
  and never closed it. `--max-age 60` closes anything idle past
  one minute.
- **End-of-day sweep.** Before signing off, run with the workspace's
  preferred floor (`--max-age 300` is a reasonable lunchtime sweep;
  `--max-age 60` is end-of-day).

## Recommended thresholds

- `--max-age 600` (default) — matches the CU pool's idle TTL.
  Safe for shared keys.
- `--max-age 300` — cost-sensitive. Reaps anything idle > 5 min.
- `--max-age 60` — aggressive. Reaps anything idle > 1 min. Use
  only when you control all E2B usage under the key.
- `--max-age 0` — close everything. **Only use under operator
  supervision after `--dry-run`** — this will close active
  sandboxes belonging to other workspaces if they share the key.

## How to confirm a sandbox was closed

1. Note the sandbox id from the reap report's `reaped` field.
2. Open the [E2B dashboard](https://e2b.dev/dashboard/sandboxes).
3. Filter to the id. Closed sandboxes show as `terminated` /
   `stopped` (the exact label depends on the template).

You can also re-run `argo sandbox e2b list` and confirm the id no
longer appears.

## Failure modes

- **`ARGO_E2B_API_KEY unset`** — export the key and retry. The
  one-shot CLI fails fast; the cron path no-ops silently.
- **HTTP 401 from E2B** — the key is invalid or revoked. Rotate
  via the E2B dashboard and re-export.
- **HTTP 429 from E2B** — rate limit hit. The reaper does not
  back-off automatically; wait one minute and retry, or split
  the reap across smaller `--max-age` windows.
- **Per-sandbox close error** — surfaced in the `errors` array
  of the report. Reaping continues past it; re-run after a minute
  to retry just the failed ids.

## Out of scope (v1)

- **Quota / budget enforcement.** The reaper is per-API-key
  cleanup, not tenant-level cost capping. Quotas land in a
  separate brief.
- **Per-template thresholds.** One threshold for all templates;
  per-template tuning is v2 if operators ask.
- **Reaping by cost rather than time.** Requires the E2B billing
  API; v2.
- **Auto-reaping inside the CU loop.** The existing `Drop` path
  is the in-band cleanup; this reaper is the out-of-band leak
  recovery path.
- **Workspace filtering.** E2B doesn't tag sandboxes with
  workspace identity. Document the tenancy boundary loudly
  rather than papering over it with a filter that won't be
  authoritative.
- **Cross-cloud sandbox reaping.** This brief is E2B-only.
  Docker / local-sandbox / future AWS-CU cleanup is its own
  surface.
