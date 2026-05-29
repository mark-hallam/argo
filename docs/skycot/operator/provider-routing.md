# Provider routing — production vs. dev/demo

SKYCOT routes every large-language-model (LLM — the AI model that
generates agent text) call through a *provider manifest*. The platform
currently ships two manifests for Claude:

| Manifest | Use case | Billing |
| --- | --- | --- |
| `manifests/providers/anthropic.toml` | Production | Anthropic developer API key (`ANTHROPIC_API_KEY`) |
| `manifests/providers/claude-code-cli.toml` | Dev / demo / build-phase | Operator's Claude Code subscription (no API key needed) |

Both manifests speak the same response shape, so consumer code
(`Pitch`, `Genesis`, the agent runner, evals, forge) is identical.
Flipping between them is a single config line.

## Why two manifests

- **Production** runs at scale and needs the deterministic, billed,
  per-token Anthropic API surface. The platform's audit + cost-router
  primitives assume real per-call billing.
- **Dev / demo / build-phase** burns through quota fast — every Pitch
  question, every Genesis generation, every worker turn during testing
  costs real money. The operator's Claude Code subscription has a
  fair-use quota model that is effectively unlimited for normal
  dev/demo volume. Routing through it saves real budget during build.

## Switching providers

There are two LLM-dispatch paths and each has its own provider selector
(there is **no** single global `[providers] default` session-provider
knob in v1 — wiring one through every session-creation site is a larger
refactor deferred past this brief):

**1. Pitch Agent (`POST /api/v1/pitch/ask`) — the demo's headline LLM
path.** Selected by the `ARGO_PITCH_LLM_PROVIDER` env var the gateway
reads at request time. Defaults to `anthropic`; set it to
`claude-code-cli` to route through the subscription:

```bash
# Dev / demo — route the Pitch Agent through the Claude Code subscription:
export ARGO_PITCH_LLM_PROVIDER=claude-code-cli

# Production (default if unset) — route through the Anthropic developer API:
# export ARGO_PITCH_LLM_PROVIDER=anthropic
```

The handler loads `manifests/providers/<id>.toml` and dispatches through
the shared `build_transport()` — a `transport_kind = "subprocess"`
manifest (claude-code-cli) goes to the subprocess driver, everything
else stays on the HTTP path. No code change in any consumer.

**2. Agent-loop turns — per-session.** A worker session's `provider`
(set at session/worker creation) names its manifest; the turn loop
already builds the transport via `build_transport()` and applies the
session's tiered routing policy. To drive the cost-tier step-down from
`argo.toml`, set `[routing] policy = "tiered"` with a `[routing.tiered]`
table (see the model-lineup section below) and create the session with
`provider = "claude-code-cli"`.

The committed template `docs/skycot/operator/argo.dev.toml.example` ships
the full dev routing config. Copy it to the **repo-root `argo.toml`** to
activate; that path is gitignored (`/argo.toml`) so local dev routing
never leaks into CI/test routing. Only the `.example` is tracked.

## Cost-model differences

| Dimension | `anthropic` (production) | `claude-code-cli` (dev/demo) |
| --- | --- | --- |
| Billing surface | Per-token, billed against Anthropic API key | Operator's Claude Code subscription (fair-use quota) |
| Per-call observability | Full — request/response logged, token counts per call | Partial — token counts are reported but billing is opaque from outside |
| Streaming | Yes (SSE — Server-Sent Events, the streaming wire format) | No (the CLI is one-shot; the platform fakes a single-chunk stream) |
| Latency overhead | ~50–100 ms HTTPS round-trip | ~500–1500 ms subprocess start + I/O |
| Tool calling | Full | Out-of-scope for v1 of this provider |

## Setup checklist

### Production — `anthropic` provider

1. Export `ANTHROPIC_API_KEY` in the environment that runs the gateway.
2. Confirm `manifests/providers/anthropic.toml` is present in the
   workspace (or under `$ARGO_MANIFESTS_DIR/providers/`).
3. Leave `ARGO_PITCH_LLM_PROVIDER` unset (defaults to `anthropic`), and
   do not place a dev `argo.toml` at the repo root.

### Dev / demo — `claude-code-cli` provider

1. Install the Claude Code CLI — see <https://docs.claude.com/en/docs/claude-code/cli>.
   Confirm `which claude` returns a binary; the operator's machine
   already has one (used by the swarm workers).
2. Optional override: `export ARGO_CLAUDE_CODE_BINARY=/path/to/claude`
   if you want to pin a specific binary version (mocks + tests use this).
3. Confirm `manifests/providers/claude-code-cli.toml` is present.
4. Copy the dev config to the repo root:
   `cp docs/skycot/operator/argo.dev.toml.example argo.toml`.
5. For the Pitch Agent path, `export ARGO_PITCH_LLM_PROVIDER=claude-code-cli`.

## What's out of scope for v1

- **Streaming** — the CLI is one-shot. v1.1 will switch to
  `--output-format stream-json` for real streaming.
- **Multi-instance subprocess pool** — one subprocess per request for
  v1. v1.1 could pool a warm CLI process for lower per-call latency.
- **Cost-tracking integration with the platform's billing surface** —
  the CLI's billing is opaque from outside (subscription, not
  per-call). Audit rows label the provider as `claude-code-cli`; the
  operator reconciles spend manually against the Claude subscription
  dashboard.
- **Auto-fallback** to `anthropic.toml` if the CLI subprocess fails.
  Explicit operator config flip is required.

## Brief

See `docs/build-plan/streams/provider-claude-code-cli-subprocess.md`
for the original design + acceptance criteria.

---

## Model lineup + tier step-down (refreshed 2026-05-29)

Current catalog (`manifests/providers/anthropic.toml`), per
[platform.claude.com models overview](https://platform.claude.com/docs/en/about-claude/models/overview):

| Model | API ID | Context | Price (in / out per MTok) | Tier label |
|-------|--------|---------|----------------------------|-----------|
| Opus 4.8 (frontier) | `claude-opus-4-8` | 1M | $5 / $25 | `complex` |
| Sonnet 4.6 (balanced) | `claude-sonnet-4-6` | 1M | $3 / $15 | `medium` |
| Haiku 4.5 (fast) | `claude-haiku-4-5` | 200k | $1 / $5 | `simple` |

The `[discovery]` block uses `prefer = "highest_minor"` with
`include_pattern = "^claude-opus-"`, so the **fixed-policy frontier
default resolves to the newest Opus (4.8)** — it will never silently
drop to Sonnet via alphabetical ordering.

### Operator directive (2026-05-29): top tasks → Opus 4.8, simpler → Sonnet 4.6

There are two ways to get this step-down, depending on which provider
is active:

**(a) Dev/demo via Claude Code CLI — explicit tier routing, Sonnet
floor.** The `claude` CLI auto-routes short prompts to **Haiku** when
no model is pinned — which the operator explicitly forbids. The driver
now passes `--model <id>` per request, so the tier table below is
authoritative and the CLI never falls back to Haiku. Copy
`docs/skycot/operator/argo.dev.toml.example` to the repo-root
`argo.toml`; its `[routing.tiered]` block is:

```toml
[routing]
policy = "tiered"

[routing.tiered]
classifier = "claude-code-cli:claude-sonnet-4-6"   # classify on Sonnet (NOT Haiku)
simple     = "claude-code-cli:claude-sonnet-4-6"
medium     = "claude-code-cli:claude-sonnet-4-6"
complex    = "claude-code-cli:claude-opus-4-8"
classify_threshold_ms = 120
fallback_to_default_on_error = true
```

**Zero Haiku anywhere** — the classifier itself runs on Sonnet 4.6, the
floor tier is Sonnet 4.6, and only `complex` rises to Opus 4.8. The
subscription absorbs the cost. This is the recommended dev/demo posture.

**(b) Production via the Anthropic API — explicit tier routing.**
To get the Opus-4.8-for-complex / Sonnet-4.6-for-simpler step-down on
the real API, set the tiered policy in the deploy's `argo.toml`
(NOT committed to the repo — it lives in the production workspace
root so it doesn't change test/CI routing):

```toml
[routing]
policy = "tiered"

[routing.tiered]
classifier = "anthropic:claude-haiku-4-5"   # cheap classifier call
simple     = "anthropic:claude-sonnet-4-6"
medium     = "anthropic:claude-sonnet-4-6"
complex    = "anthropic:claude-opus-4-8"
classify_threshold_ms = 80
fallback_to_default_on_error = true
```

The classifier runs Haiku once per request to label it
`simple|medium|complex`, then routes to the mapped model. Per the
operator directive, both `simple` and `medium` step down to Sonnet
4.6; only `complex` rides Opus 4.8. (Production tolerates Haiku for the
cheap classifier call; the dev path above does not.)

**Default (no `[routing]` block) is `policy = "fixed"`** — every call
uses the frontier default (Opus 4.8). Simplest + most capable, but
most expensive. Fine for low-volume or when every call benefits from
the top model.
