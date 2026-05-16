# SKYCOT developer docs

The customer-facing path starts at `skycot.com` → signup →
[`../getting_started.md`](../getting_started.md). The pages below
are for engineers and contributors running SKYCOT against the
local `argo` binary or extending the platform.

## Run SKYCOT locally

- [`cli-getting-started.md`](cli-getting-started.md) — install the
  binary, init a workspace, queue a worker job, drive it through
  the approval ledger from the terminal.
- [`../provider_keys.md`](../provider_keys.md) — Anthropic /
  OpenAI key management; systemd, Kubernetes Secret, Vault patterns.
- [`../operator_runbook.md`](../operator_runbook.md) — full
  `argo worker actions` CLI reference + ledger artifact map.

## Extend the platform

- [`../../plan/argo-build-plan.md`](../../plan/argo-build-plan.md) —
  master spec for the core platform.
- [`../../design/worker_v3_execution.md`](../../design/worker_v3_execution.md) —
  the generic execution layer that vertical packs plug into.
- [`../PLUGIN_SYSTEM.md`](../PLUGIN_SYSTEM.md) — JSON-RPC plugin ABI.
- [`../../../CONTRIBUTING.md`](../../../CONTRIBUTING.md) — how to
  open a PR.

## Verify a build

- `./scripts/smoke-skycot.sh` — mock-mode end-to-end smoke; no
  API keys required.
- `./scripts/smoke-skycot-real-llm.sh` — same path against a real
  provider; auto-skips when no key is set.
