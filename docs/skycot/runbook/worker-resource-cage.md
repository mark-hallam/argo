# Per-worker resource cage (cgroup-v2)

Operator runbook for `argo-runtime::worker_resource_cage`
(mission-cgroup-per-worker-resource-limits, MISSION_BACKLOG §8
#236 — Tier P reliability).

## What it does

For every worker subprocess on a Linux host with cgroup-v2
mounted, the supervisor creates a slice at
`/sys/fs/cgroup/argo-worker-<worker_id>/` and writes:

- `memory.max` ← `memory_max_mib * 1MiB` (default **1024 MiB**)
- `cpu.weight` ← `cpu_weight` (default **100**, range `1..=10000`)
- the worker pid into `cgroup.procs`

A runaway worker that breaches `memory.max` is OOM-killed by the
kernel; the [oom-restart handler](oom-restart.md) catches the
exit, audits it, and decides whether to respawn.

## Platform requirements

- **Linux** with cgroup-v2 mounted unified (`grep cgroup2
  /proc/mounts` shows `/sys/fs/cgroup`). cgroup-v1 returns
  `NotSupported`; no v1 backport.
- The argo process must have write access to `/sys/fs/cgroup`.
  On systemd hosts, run argo under `systemd-run --user --scope
  --slice=argo` to get a writable sub-slice.
- macOS / Windows: returns `NotSupported`; use Docker or Lima.

## Disable per host or per tenant

`argo.toml`:

```toml
[runtime.worker_resource_cage]
enabled = false
```

To disable globally for a fleet, set the same flag in the operator-
managed config. Per-tenant caps are out of scope here — that's
`mission-tenant-resource-aggregate` (#239).

## Inspect current caps

```bash
cat /sys/fs/cgroup/argo-worker-<worker_id>/memory.max
cat /sys/fs/cgroup/argo-worker-<worker_id>/cpu.weight
cat /sys/fs/cgroup/argo-worker-<worker_id>/memory.current
cat /sys/fs/cgroup/argo-worker-<worker_id>/memory.events
```

`memory.current` is the live RSS; `memory.events`'s `oom_kill`
counter increments every time the kernel reaps the worker.

## macOS dev hosts

Native cgroup-v2 is Linux-only. On macOS, the cage returns
`NotSupported { reason: "cgroup-v2 requires Linux" }`. For local
confinement: run inside Docker (`docker run --memory=1g --cpus=1
...`) or a cgroup-v2-mounted Lima VM (`limactl start --name=argo
template://ubuntu`). Inside the container, `enabled = true` lets
the cage write the cgroup files normally.

## Tuning

| Workload                              | Knob to turn                  |
| ------------------------------------- | ----------------------------- |
| Browser computer-use (Chrome RSS)     | Raise `memory_max_mib` to 2048+ |
| Low-priority batch backfill           | Lower `cpu_weight` to 25-50   |
| Realtime / interactive verticals      | Raise `cpu_weight` to 200+    |
| Repeated OOM thrashing                | See [oom-restart](oom-restart.md) escalation table |

## See also

- Brief: `docs/build-plan/streams/mission-cgroup-per-worker-resource-limits.md`
- Downstream: [oom-restart.md](oom-restart.md)
- Follow-up wire-up: `mission-worker-cage-wire-spawn`
- Per-tenant aggregates: `mission-tenant-resource-aggregate` (#239)
