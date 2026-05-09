# Tracing Span Naming

Span names in this workspace follow a single canonical hierarchy so
trace dashboards stay searchable, span filters never need OR clauses
across naming variants, and new contributors copy the right shape by
default.

## Contract

```
<domain>.<subdomain>[.<action>]
```

- All-lowercase. Each segment is `snake_case`.
- Two or three dotted segments, no more.
- Validated by regex: `^[a-z_]+\.[a-z_]+(\.[a-z_]+)?$`.
- The `argo_telemetry::spans` module in
  `crates/argo-telemetry/src/lib.rs` is the **single source of truth**
  for declared span names. Add a span name there before using it.

## Top-level domains

Use one of:

`agent`, `cli`, `channels`, `forge`, `gateway`, `judge`, `marketplace`,
`mcp`, `memory`, `provider`, `routing`, `session`, `skill`, `store`,
`tui`, `vertical`, `worker`.

If a span obviously belongs to a domain not in this list, **add the
domain to this doc** in the same PR — do not invent an alternative
shape.

## Subdomain (second segment)

The noun the span operates on: `session`, `message`, `route`,
`manifest`, `action`, `event`, `turn`, `context`, `tools`, `provider`,
`run`, `replay`, `push`, `pull`, etc.

## Action verb (third segment, optional)

When present, prefer one of these standard verbs:

`create`, `read`, `list`, `update`, `delete`, `dispatch`, `validate`,
`resolve`, `emit`, `consume`, `execute`, `request`, `stream`, `plan`,
`build`, `classify`, `adjudicate`, `persist`.

For background-loop or periodic-tick spans, use `tick` or `loop` as
the action segment (e.g. `worker.reaper.tick`).

## Examples

| Span name                  | Where it fires                   |
| -------------------------- | -------------------------------- |
| `agent.turn`               | One iteration of the agent loop  |
| `agent.context.build`      | System prompt + memory assembly  |
| `agent.provider.request`   | Outbound model call              |
| `agent.provider.stream`    | Streaming response consumer      |
| `agent.tools.plan`         | Tool dispatch planning           |
| `agent.tools.execute`      | Tool execution                   |
| `agent.persist`            | Final state persistence          |
| `routing.classify`         | Cost-router tier decision        |
| `judge.adjudicate`         | Judge verdict                    |
| `forge.run`                | One Forge optimization run       |
| `session.replay`           | Time-travel replay               |
| `marketplace.push`         | Workspace bundle upload          |
| `marketplace.pull`         | Workspace bundle download        |

## How spans are emitted

Every span goes through the `TelemetrySink` trait
(`crates/argo-telemetry/src/sink.rs`):

```rust
let _g = sink.span(spans::AGENT_TURN, &[("session", session_id)]);
```

Where `spans::AGENT_TURN` is a `&'static str` constant declared in
`argo_telemetry::spans`. Callers must use these constants — never
inline a span name as a string literal at the call site, because that
bypasses the central registry and the CI guard below.

The single underlying `tracing::info_span!` lives in
`crates/argo-telemetry/src/tracing_sink.rs` and uses the literal
`"argo"` as the static span name required by `tracing`, with the
canonical span name carried in the `span` field. This wrapper is the
only acceptable `tracing::*_span!` call site in the workspace and is
exempted from the regex check.

## CI guard

`scripts/check-span-naming.sh` runs in CI (`clippy` job in
`.github/workflows/ci.yml`). It enforces:

1. Every `pub const … : &str = "…"` declared in
   `crates/argo-telemetry/src/lib.rs`'s `spans` module matches the
   regex.
2. No `tracing::*_span!("literal")` or
   `#[instrument(name = "literal")]` exists outside the
   `argo-telemetry` crate (forces all spans through the registry).

Run locally:

```bash
scripts/check-span-naming.sh
```

## Adding a new span

1. Pick the domain. Reuse one from the list above; if none fits, add
   it to this doc.
2. Add a `pub const NEW_SPAN: &str = "domain.subdomain.action";` to
   `argo_telemetry::spans` and include it in `spans::all()`.
3. Use the constant at the call site:
   `sink.span(spans::NEW_SPAN, &[…])`.
4. Run `scripts/check-span-naming.sh` locally.

## Trace-dashboard impact

Dashboards consuming earlier ad-hoc names will keep working; this doc
codifies the existing canonical hierarchy that `argo_telemetry::spans`
already enforces. If a new span replaces an older display name in the
future, flag the rename in the PR description so dashboard owners can
update.
