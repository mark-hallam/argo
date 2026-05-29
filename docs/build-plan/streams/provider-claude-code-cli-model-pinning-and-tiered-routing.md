# provider-claude-code-cli-model-pinning-and-tiered-routing

**Priority**: P0 — dev/demo cost-control: route LLM via Claude Code subscription with explicit per-tier model pinning (Sonnet floor, Opus for complex, never Haiku)
**Source**: 2026-05-29 operator direction (extends #2663 Claude Code CLI provider)
**Authored against**: origin/main `2736f887b`
**Authored by**: claude conductor (interactive session, operator overnight/active authority)

## Why this brief exists

#2663 shipped the `claude-code-cli` subprocess provider so LLM calls can route through the operator's Claude Code subscription instead of the metered Anthropic API. Two gaps remain before the operator's intended dev/demo routing works:

1. **The driver doesn't pass `--model` per request.** `ClaudeCodeCliTransport::invoke_cli(prompt)` runs `claude --print --output-format json <prompt>` with no model arg, so the CLI auto-routes — and auto-routing picks Haiku for short/simple prompts. **The operator explicitly does NOT want Haiku; Sonnet 4.6 is the floor.** Without per-request `--model`, the tier router's model choice is silently ignored by the CLI.

2. **The `[providers] default` provider-selection knob is unverified.** `docs/skycot/operator/provider-routing.md` (shipped by #2663) tells operators to set `[providers] default = "claude-code-cli"` in `argo.toml`, but the config loader's provider-selection path isn't confirmed wired — `ProvidersConfig` is `[provider.*]` breaker tables, not a `default` selector. Either the knob exists under a different name, or it needs wiring so `argo.toml` can actually flip the active provider.

This brief closes both so the operator gets: **Claude Code subscription routing + Sonnet 4.6 floor + Opus 4.8 for complex, never Haiku.**

## Scope

### 1. Per-request `--model` in the CLI driver

`crates/argo-provider/src/claude_code_cli/driver.rs`:
- Thread the requested model id from the `CompleteRequest` (or whatever the `Transport::complete` signature carries) into `invoke_cli`.
- When a model is specified, append `--model <id>` to the spawned command args (after `default_args`, before the prompt).
- When no model is specified, preserve current behavior (CLI auto-routes) for backward compat.
- Map the platform model id to whatever the `claude` CLI expects (`claude-sonnet-4-6`, `claude-opus-4-8` — these are the CLI's accepted `--model` values; the CLI accepts the dateless aliases).
- Add unit tests via the mock binary: assert `--model claude-sonnet-4-6` is present in the spawned args when the request pins Sonnet; assert absence when unpinned.

### 2. Verify/wire the provider-selection default

Investigate how the gateway resolves the active provider + default model at boot (`crates/argo-cli/src/handlers/gateway/boot*.rs`, `crates/argo-routing/src/`, `crates/argo-config/`). Then:
- If a `[providers] default` (or equivalent) knob already exists and works → document the exact key + confirm with a test.
- If it does NOT exist → add a minimal config field (e.g. `[routing] default_provider = "..."` or extend the existing routing config) so `argo.toml` can select `claude-code-cli` as the active provider for all LLM dispatch.
- The FixedRouter's `default_model: ProviderModel` is the lever — ensure it can be set from `argo.toml` to `claude-code-cli:claude-sonnet-4-6`.

### 3. Tiered routing config (Sonnet floor, Opus complex, no Haiku)

The `[routing.tiered]` config (already defined in `crates/argo-config/src/sections_routing.rs`) maps `simple|medium|complex` → `provider:model`. Wire a dev config:

```toml
[routing]
policy = "tiered"

[routing.tiered]
classifier = "claude-code-cli:claude-sonnet-4-6"   # classify via Sonnet (NOT Haiku)
simple     = "claude-code-cli:claude-sonnet-4-6"
medium     = "claude-code-cli:claude-sonnet-4-6"
complex    = "claude-code-cli:claude-opus-4-8"
classify_threshold_ms = 120
fallback_to_default_on_error = true
```

**No Haiku anywhere** — the classifier itself runs on Sonnet, and the floor tier is Sonnet. Only `complex` rises to Opus 4.8.

### 4. Ship a gitignored dev `argo.toml` at repo root

The gateway reads `argo.toml` from its cwd (repo root `~/code/argo`). To activate this routing for the operator's local gateway WITHOUT changing CI/test routing:
- Add `/argo.toml` to `.gitignore` (repo-root config stays local-only).
- Ship a committed template at `docs/skycot/operator/argo.dev.toml.example` with the full dev routing config above + a header comment: "copy to repo-root `argo.toml` to activate Claude Code subscription routing for local dev/demo; never commit the active argo.toml."
- The operator (or this brief's verification step) copies the example to repo-root `argo.toml`.

### 5. Verify end-to-end

- Restart the gateway with the dev `argo.toml` in place + `claude` CLI on PATH.
- `POST /api/v1/pitch/ask` with a novel question → SPA `source: "llm"`, response present, and the gateway log / audit shows the call routed through `claude-code-cli` on **Sonnet 4.6** (not Haiku).
- Trigger a complex agent turn (or a classifier that returns `complex`) → routed to **Opus 4.8**.
- Confirm no call ever lands on Haiku.
- Document the verification result in the PR body (real `claude` CLI smoke, like #2663 did).

## Acceptance

- [ ] CLI driver passes `--model <id>` per request; mock-binary tests assert the arg
- [ ] Provider-selection knob confirmed/wired so `argo.toml` activates `claude-code-cli`
- [ ] `[routing.tiered]` dev config maps simple/medium → Sonnet 4.6, complex → Opus 4.8, classifier → Sonnet (zero Haiku references)
- [ ] `.gitignore` excludes repo-root `/argo.toml`; committed example at `docs/skycot/operator/argo.dev.toml.example`
- [ ] Real-binary smoke: a Pitch question routes through the CLI on Sonnet 4.6; a complex turn routes to Opus 4.8; Haiku never selected
- [ ] `cargo clippy -p argo-provider -p argo-routing -p argo-config --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-provider -p argo-routing -p argo-config` green (incl. new model-pinning + selection tests)
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean
- [ ] No regression: existing provider/routing tests pass; `anthropic` (API) path unchanged for production

## Out of scope

- Real streaming through the CLI (still fake-stream per #2663)
- Per-tenant routing overrides (single workspace config for v1)
- Cost-tracking reconciliation against the subscription dashboard (manual)
- Changing the production (`anthropic.toml`) routing — production stays API + Opus-4.8 frontier default

## May-touch

- `crates/argo-provider/src/claude_code_cli/driver.rs` + `tests.rs` (per-request `--model`)
- `crates/argo-provider/src/claude_code_cli/mod.rs` (if signature plumbing needed)
- `crates/argo-routing/src/` (FixedRouter default_model from config, if selection wiring needed)
- `crates/argo-config/src/sections_routing.rs` + `provider.rs` (provider-default selector, if not already present)
- `crates/argo-agent-runner/src/transport.rs` (thread model into subprocess dispatch)
- `.gitignore` (exclude `/argo.toml`)
- `docs/skycot/operator/argo.dev.toml.example` (NEW committed template)
- `docs/skycot/operator/provider-routing.md` (update the step-down section to reflect real wiring)

## Operator authority

`--skip-clippy` lane authorized (operator active direction 2026-05-29); full worker-side verification required incl. real-binary smoke. Sonnet 4.6 is the hard floor — any config or test that references Haiku in the dev routing path is a brief violation.
