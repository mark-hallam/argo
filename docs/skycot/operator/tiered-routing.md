# Tiered routing — per-turn model selection by classifier verdict

Status: v1.x. Cost-tier half of mission claim 3 PROVEN with
`mission-claim-3-runtime-wiring-v1x`. Inter-provider fallback is a
sibling brief (`mission-provider-fallback-on-error-v1x`) and is NOT
covered here.

## What tiered routing does

Every session carries a `model` column (e.g. `claude-sonnet-4-6`) that
the agent loop uses by default. When a session opts into
`RoutingPolicy::Tiered`, the agent loop classifies each user message
against a cheap `classifier_model` before the main provider call,
then routes to one of three configured tier models:

- **simple** — short / chitchat / lookup-style prompts
- **medium** — moderate planning, single-step tasks
- **complex** — multi-step reasoning, full agentic chains

The classifier returns a single word; the agent loop swaps in that
tier's `(provider, model)` for the dispatched turn. If the classifier
errors out and `fallback_to_default_on_error = true` (the
recommended default), the turn falls back to `session.model` so the
loop never halts.

A `routing.decision` audit row lands on every routed turn with the
session id, turn number, tier, selected model, provider, and
classifier latency in milliseconds — read it via:

```
argo audit by-session <session-id> --kind routing.decision
```

Sessions with `routing_policy = NULL` (the default for every existing
row at upgrade time) skip this path entirely and behave identically
to pre-v1.x.

## Setting a tiered policy on a session

```
argo session set-routing-policy <session-id> --tiered \
  --classifier-provider openai     --classifier-model gpt-4o-mini \
  --simple-provider openai         --simple-model gpt-4o-mini \
  --medium-provider openai         --medium-model gpt-4o \
  --complex-provider anthropic     --complex-model claude-sonnet-4-6 \
  --classify-threshold-ms 500      --fallback-to-default-on-error true
```

To clear the policy and return the session to its default model:

```
argo session set-routing-policy <session-id> --none
```

Both forms support `--json` for scripting.

## Wire shape (for SKYCOT engineers)

1. `RoutingPolicy` config lives on the session row as JSON in
   `session.routing_policy` (slot 117). NULL means "no policy".
2. `argo_agent_runner::turn::run_turn_with_events_inner` reads the
   policy off the session header.
3. When the policy is `RoutingPolicy::Tiered`,
   `argo_agent_runner::turn_routing::maybe_route_turn` builds a
   classifier `Transport` from the session's provider manifest and
   constructs `argo_routing::TieredRouter`. The router runs the
   `§24.2` classifier prompt, parses the verdict, and returns a
   `RoutingDecision` whose `chosen_provider_model.model` overrides
   `session.model` for the dispatched turn.
4. The audit sink writes a `routing.decision` row capturing the
   decision; the existing `routing_decision` table population is
   handled by callers that need the typed read-side (e.g.
   `argo route inspect`).

## Operator interpretation of `routing.decision` audit rows

Each `routing.decision` payload carries:

| Field         | Meaning                                                 |
|---------------|---------------------------------------------------------|
| `session_id`  | Session the decision was made on                        |
| `turn`        | 1-based count of user turns when the decision fired     |
| `tier`        | `simple` / `medium` / `complex` / `unrouted`            |
| `model`       | The model the agent loop dispatched the turn against    |
| `provider`    | Provider matching the manifest used for that model      |
| `elapsed_ms`  | Classifier round-trip latency in milliseconds           |

`tier = "unrouted"` indicates the classifier timed out, errored, or
returned a value outside the tier set; with the recommended
`fallback_to_default_on_error = true` the agent loop kept
`session.model` and recorded the decision so the operator can
observe classifier drift.

## What's NOT in scope here

- Inter-provider fallback on `Transport::complete` error (sibling
  brief `mission-provider-fallback-on-error-v1x`).
- Cost-tier priority routing (`tier-1 for $$$, tier-3 for batch`) —
  add a follow-up brief if a tenant needs it.
- Manifest-side tier metadata (tier configuration lives on
  `RoutingPolicy::Tiered`, NOT in `manifests/providers/*.toml`).
