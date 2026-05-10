# Structured logging runbook

Field convention for the structured-tracing spans the gateway,
agent-runner, and work-loop wrap around their request / per-turn
/ per-job / per-step bodies. Every `tracing::info!` / `warn!` /
`error!` emission inside one of these scopes inherits the
ancestor span fields without a per-callsite change — that is the
contract operators rely on for log search.

Locked by the `mvp-structured-logging` brief and the smoke test
at `crates/argo-cli/tests/structured_logging_smoke.rs`. A
regression there is a wire-shape change for whoever is consuming
your logs.

## Span tree (production stack)

A typical end-to-end emission inside a worker-driven turn nests
four spans:

```
worker_job{tenant_id, vertical_id, job_id}
└── step{step, max_steps}
    └── turn{session_id, tenant_id, worker_id}
        └── (provider call, tool dispatch, audit write, …)
```

For a gateway-driven turn the outer wrapper is `request` instead:

```
request{route, principal_id, request_id}
└── turn{session_id, tenant_id, worker_id}
    └── (handler body)
```

`turn` is opened in
`crates/argo-agent-runner/src/turn.rs::run_turn_with_events`;
`worker_job` in
`crates/argo-cli/src/handlers/work.rs::serve`; `step` in
`crates/argo-cli/src/handlers/work_loop.rs::run_loop`; `request`
in
`crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs::dispatch_post`.

## Field convention (snake_case, one canonical name each)

| Field          | Type     | Where it's set                       |
|----------------|----------|--------------------------------------|
| `tenant_id`    | UUID     | `turn` (after session load), `worker_job` |
| `worker_id`    | UUID     | `turn` (after session load) — empty when the session has no worker bound |
| `session_id`   | UUID     | `turn`                               |
| `request_id`   | string   | `request` — caller-supplied via `X-Request-Id`; empty when absent |
| `route`        | string   | `request` — `<verb> <path>`, e.g. `POST /sessions` |
| `principal_id` | string   | `request` — resolved from `X-Argo-Principal` (`worker:<uuid>`, `human:<id>`, `system:<id>`); empty when anonymous |
| `vertical_id`  | string   | `worker_job`                         |
| `job_id`       | UUID     | `worker_job`                         |
| `step`         | u32      | `step` — 1-based step index          |
| `max_steps`    | u32      | `step` — configured ceiling          |

`audit_kind` is the one extra field the brief reserves for audit
emissions made directly via `tracing::info!(target =
"argo::*::audit", audit_kind = …, …)`. It is NOT span-inherited;
it is per-event. Existing audit call sites already use it.

## How to consume

Default `tracing-subscriber` text format renders span fields as
`name{field=value}` segments before the event message:

```
INFO worker_job{tenant_id=… vertical_id=… job_id=…}:step{step=1 max_steps=3}:turn{session_id=… tenant_id=… worker_id=…}: dispatched
```

For JSON output, swap in
`tracing_subscriber::fmt().json()`. Span fields land on each
emission as flat top-level keys.

## How to add a new span

1. Pick a name from the table above OR coin a new snake_case
   noun. Don't invent synonyms — `tenant`, `tenantId`,
   `tenant-id` all break log search.
2. Use `tracing::info_span!("…", field = %value, …)` for fields
   known up front, or
   `tracing::info_span!("…", field = tracing::field::Empty)`
   plus
   `tracing::Span::current().record("field", tracing::field::display(&value))`
   for fields that resolve later in the body.
3. For async bodies use `.instrument(span).await` from
   `tracing::Instrument` — `let _enter = span.enter();` only
   works for synchronous code. The agent-runner's `turn` span
   uses `.instrument()` for exactly this reason.
4. Add a case to
   `crates/argo-cli/tests/structured_logging_smoke.rs` so the
   field name + value rendering is locked.

## What does NOT belong on a span

- Per-event detail (an HTTP response code, a single tool name, a
  cost in cents). These belong as fields on the emission itself
  (`tracing::info!(status = 200, …)`). Span fields are ambient
  context that's true for every emission inside the scope.
- Secrets. The whole emission can flow into long-term log
  storage; do not put bearer tokens, raw principal headers, or
  full tool-call payloads on a span.
- Mutable state. Span fields are recorded once. Use a `Span ::
  current().record(…)` write for late-bound resolution but
  don't try to model state machines through span field churn.

## Failure modes & recovery

- **Missing field on emissions.** A handler opened a span but
  skipped a canonical field. Audit the
  `tracing::info_span!(…)` invocation against the table above.
- **Duplicate field names.** Two ancestor spans both set
  `tenant_id` to different values. Default text formatter prints
  both; JSON formatter keeps the inner one. Don't set the same
  field at two layers — the agent-runner's `turn` span is the
  authoritative source for `tenant_id` in production.
- **Span not visible in output.** The subscriber's
  `with_max_level` is below `INFO`. Bump
  `RUST_LOG=argo=info,argo_cli=info,argo_agent_runner=info` and
  retry.
- **Async span not propagating.** The body uses `let _enter =
  span.enter();` instead of `.instrument(span).await`. The
  former drops the guard at the first `.await` suspension. Fix:
  switch to `.instrument()`.
