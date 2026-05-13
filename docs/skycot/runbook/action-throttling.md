# Action throttling

Some action types — `send_email`, `post_listing`, `create_invoice`
— have natural per-minute rate limits at the downstream API.
Without a worker-side gate, dispatch fires as fast as the loop
spins, hits 429s, then burns retry budget on avoidable load.

`mission-per-action-type-throttling` adds a token-bucket gate at
the dispatch layer. The bucket is keyed by
`(tenant_id, action_type)` — one slow tenant cannot starve
another tenant's dispatch.

## How it works

- One [`ThrottleRegistry`] per supervisor / dispatcher.
- For each dispatch attempt, the runtime calls
  `try_acquire(tenant_id, action_type, Utc::now())`.
- `Proceed` → fire the action immediately.
- `Wait { delay }` → yield to the scheduler for `delay` and retry.
  No busy-wait — the gate stays at the dispatch layer.
- When the cumulative wait observed across one dispatch's poll
  loop crosses `audit_threshold_secs` (default 10s), the dispatcher
  emits a `runtime.action_throttled` audit row with payload
  `{ action_type, queued_for_secs, tenant_id }`.

## Operator knobs (`argo.toml`)

```toml
[runtime.action_throttle]
enabled = true              # master enable, default false
audit_threshold_secs = 10   # emit audit row when wait ≥ this

[runtime.action_throttle.policies.send_email]
tokens_per_sec = 5.0
burst = 10

[runtime.action_throttle.policies.post_listing]
tokens_per_sec = 2.0
burst = 3
```

Disabled by default — the runtime dispatch path is zero-overhead
until `enabled = true`. Per-action policies are opt-in; an action
type absent from the `policies` table dispatches without
throttling even when the master enable is on.

## Inspection

```text
$ argo doctor throttles
argo doctor throttles — per-action-type throttle gate
  status:            ENABLED
  audit threshold:   10s (runtime.action_throttled when wait ≥)
  policies (2):
    - post_listing: 2 tok/s, burst 3
    - send_email: 5 tok/s, burst 10
  buckets:           (no live state — registry inspection is CLI-snapshot only in v1)
```

`argo doctor throttles --json` is the contract; the text form is
operator convenience.

## Sustained throttling means under-tuned config

A `runtime.action_throttled` row firing once is normal — bursts
exceed steady-state by design. A *sustained* stream of audit rows
at the same `(tenant_id, action_type)` means `tokens_per_sec` is
too tight for the workload. Either raise the rate, raise `burst`,
or accept the queueing and watch the dispatch latency rise.

## What this is NOT

- **Not a per-action-instance rate limiter.** v1 throttles by
  type, not by recipient (per-prospect / per-customer). That
  would multiply bucket count by N and is a different brief.
- **Not a priority queue.** FIFO in v1 — every blocked dispatch
  gets the same `Wait` treatment regardless of caller.
- **Not multi-host coordinated.** Single-host coverage. Distributed
  rate-limiting (Redis-token or equivalent) is the zero-trust
  brief's surface, not this one.
