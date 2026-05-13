# Sandbox pre-warming

E2B sandbox creation has a cold-start cost (~10-15s). When a
vertical fires a burst of sandboxed actions in succession, each
cold-start chains. The
`argo-executor-computer-use::sandbox_prewarm` module keeps a
small reservoir of pre-opened sandboxes during high-activity
windows so the next acquire returns instantly.

## When to enable

Demo, real-time, and customer-facing workloads — anywhere a
multi-second wait between actions is visible to the operator or
end-user. Nightly-batch tenants should leave this off: the
pre-warm reservoir consumes E2B-minutes even when unused, and
batch latency hides cold-start cost in queue time anyway.

## Triggers

Two triggers drive admission to the reservoir, both bounded by
`max_prewarm`:

1. **Sliding-window spawn rate.** Every demand for a sandbox
   (slot miss + reservoir miss) lands in a sliding window
   (`window_seconds`, default 300s). When the count *strictly
   exceeds* `spawn_rate_threshold` (default 3), the prewarmer
   opens `burst_size` (default 2) extra sandboxes in the
   background.
2. **Hourly tick during operator-active hours.** Once per active
   hour the prewarmer opens one sandbox. Active hours are the
   inverse of the operator's quiet-hours block; empty
   `active_hours` means "every hour is active".

## Configuration

```toml
[runtime.sandbox_prewarm]
enabled = false                 # opt-in
spawn_rate_threshold = 3        # spawns/window above which burst fires
max_prewarm = 5                 # reservoir cap
window_seconds = 300            # sliding-window width
burst_size = 2                  # extra sandboxes per burst
idle_ttl_seconds = 600          # reservoir reaper TTL
active_hours = []               # empty = every hour active
```

## Audit trail

The prewarmer emits two structured-log kinds via `tracing`:

- `runtime.sandbox_prewarmed` — one per sandbox admitted to the
  reservoir. Fields: `template`, `sandbox_id`, `occupancy`.
- `runtime.sandbox_prewarm_consumed` — one per FIFO take of a
  pre-warmed sandbox by dispatch. Fields: `template`,
  `sandbox_id`, `occupancy`.

A `runtime.sandbox_prewarm_open_failed` warn-row surfaces when
the background open call fails — pre-warming is best-effort,
never a correctness requirement.

## Pairing with the sandbox-pool reaper

Pre-warming creates; the out-of-band reaper
(`argo-executor-computer-use::e2b_reap_cron`) destroys idle
sandboxes E2B-side. The two balance: an over-eager pre-warm
threshold burns wallet, an over-aggressive reaper drops
just-warmed sandboxes before they're used. The shipped defaults
(threshold 3, reservoir 5, idle TTL 600s) leave comfortable
headroom for demo-cadence workloads. Operators tuning either
knob should watch both audit streams together.

## What this is NOT

- **Not ML-driven.** The trigger model is a sliding-window
  heuristic + an hourly tick. No prediction, no anomaly
  detection. If your traffic doesn't fit "bursty around active
  hours," disable this and let the in-band pool handle cold opens.
- **Not multi-tenant-budgeted.** v1 uses a single global
  reservoir cap. Per-tenant pre-warm budgets are a follow-up.
- **Not for non-E2B sandbox types.** Wayland and native sandboxes
  have different lifecycle costs; this brief covers E2B only.
