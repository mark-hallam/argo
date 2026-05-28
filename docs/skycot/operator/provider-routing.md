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

Set the routing default in `argo.toml` (or `.argo/env.local`):

```toml
[providers]
# Dev / demo — route through the Claude Code subscription:
default = "claude-code-cli"

# Production — route through the Anthropic developer API:
# default = "anthropic"
```

When the gateway picks a provider for an LLM completion, it reads this
default and dispatches to the named provider. No code change is needed
in any consumer.

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
3. Set `default = "anthropic"` in `argo.toml`.

### Dev / demo — `claude-code-cli` provider

1. Install the Claude Code CLI — see <https://docs.claude.com/en/docs/claude-code/cli>.
   Confirm `which claude` returns a binary; the operator's machine
   already has one (used by the swarm workers).
2. Optional override: `export ARGO_CLAUDE_CODE_BINARY=/path/to/claude`
   if you want to pin a specific binary version (mocks + tests use this).
3. Confirm `manifests/providers/claude-code-cli.toml` is present.
4. Set `default = "claude-code-cli"` in `argo.toml`.

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
