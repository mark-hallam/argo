<p align="center"><strong>Argo</strong> — the ship of agents</p>

# Argo

Argo is the agent framework that **a single engineer can fully audit on a weekend** and that **a 200-person company can deploy to production** without writing the multi-tenant layer themselves.

> **Status: v1 demo gate green (9/9 e2e steps); production-readiness checklist tracked in [`docs/build-plan/MISSION_BACKLOG.md`](docs/build-plan/MISSION_BACKLOG.md) §2.** Phases 0–7 + 4.5 + 6.5 + 7.5 all shipped across **930+ merged PRs** on `main` (snapshot 2026-05-07; `gh pr list --state merged --limit 1000` is the source-of-truth). The gateway catalog (`GET /routes`) lists **143 endpoints** (97 GET + 46 POST) with universal `--json` + `?format=markdown`. Substrate primitives (Worker / Persona / Role, capability engine, MCP wire, computer-use loop, 8 channel impls, judge crate, A/B + skill quality, audit chain, OCI distribution) are feature-complete; runtime closed-loops landed across the May Phase A wires (judge in skill creation #936 + forge promote #940, memory hydration #945, outbound notify on Ask #931, MCP action executor #933, duty-scheduler in gateway boot #939, gateway bearer auth #946).
>
> **Honest gap.** The deep-research pass at tick 198 framed substrate ≈ 75% / runtime ≈ 45% / mission-spirit ≈ 45%; the May Phase A wires close most of the named runtime gaps (see [`docs/skycot/MISSION.md`](docs/skycot/MISSION.md) for the refreshed snapshot). What's still in flight: per-Principal audit-row stamping on the new bearer-auth gate, Action-level ApiExecutor (mission-wire-api-action-executor BLOCKED on schema), per-worker memory scope (workspace-shared shipped in #945), and the docs-only headline reconciliation across launch_readiness.md.

## The five non-negotiable invariants

1. **Polyglot core.** Rust core + JSON-RPC plugin ABI. Plugins are subprocesses.
2. **Multi-tenant by construction.** `tenant_id` plumbed through every primitive.
3. **Capability-based security with first-class RBAC.** Every dispatch checks declared capabilities.
4. **Provider manifests, not adapters.** Quirks live in TOML, not Rust.
5. **MCP-first integration.** 8 in-core tools; everything else is an MCP server.

These five are the spec. Anything that violates them is a bug.

## Running a worker (SKYCOT)

Argo also ships **SKYCOT** — the commercial worker layer for running role-bound, evidence-aware, approval-gated AI workers. If you came here to operate a worker (not extend the platform), start with the operator-facing docs:

- [`docs/skycot/README.md`](docs/skycot/README.md) — what SKYCOT is + safety boundaries.
- [`docs/skycot/getting_started.md`](docs/skycot/getting_started.md) — 10-minute path from install to your first approved + executed action.
- [`docs/skycot/demo_script.md`](docs/skycot/demo_script.md) — marketplace-seller end-to-end demo.
- [`docs/skycot/operator_runbook.md`](docs/skycot/operator_runbook.md) — full `argo worker actions` CLI reference + ledger artifact map.
- [`scripts/smoke-skycot.sh`](scripts/smoke-skycot.sh) — end-to-end mock-mode smoke test (no API key needed); companion [`scripts/smoke-skycot-real-llm.sh`](scripts/smoke-skycot-real-llm.sh) drives the same path against a real provider (Anthropic or OpenAI), skip-with-exit-0 when no key is set.

The launch vertical is `verticals/gtm-engineer/` (marketplace seller). The Worker v3 generic execution layer lives in `crates/argo-cli/src/handlers/worker_actions*.rs`; the design spec is `docs/design/worker_v3_execution.md`.

## Why not Hermes Agent?

Argo is a structurally different shape: Rust core, JSON-RPC plugin ABI, multi-tenant by construction, capability-based security, provider manifests instead of adapters, an MCP-first integration model — plus an external-judge adjudicator for skill creation, automated skill optimization (Argo Forge), time-travel session replay, and a workspace OCI marketplace. Roughly 5× smaller for ~95% of Hermes capability and addressing the 14 most-cited Hermes user pain points by design (token overhead, RBAC, multi-agent, adaptive timeouts, …). See `docs/plan/argo-vs-hermes-gap-analysis.md`.

## Install

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.sh | bash
argo setup
```

**Windows** (PowerShell — built into Windows 10/11):

```powershell
irm https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.ps1 | iex
argo setup
```

Both installers:
- Detect your OS + architecture
- Fetch a pre-built binary from the latest GitHub release (~12 MB)
- Drop it in a user-scope dir (no sudo / admin required) — `~/.local/bin/argo` on unix, `%LOCALAPPDATA%\argo\bin\argo.exe` on Windows
- Add that dir to your PATH if it's missing
- Tell you to run `argo setup` to continue

`argo setup` walks you through workspace init + provider key configuration.

Pin a specific version with `ARGO_VERSION=v0.1.0` or change the destination with `ARGO_INSTALL_DIR=...`. Both env vars work on both installers.

### Drive it offline (no API key required)

```bash
argo gateway start &           # bind 127.0.0.1:7400
curl http://127.0.0.1:7400/    # landing JSON + entry-point hints
curl http://127.0.0.1:7400/routes  # the full 143-endpoint catalog
argo doctor                    # workspace health probe
argo gateway probe             # K8s-style readiness probe
argo tui                       # full Ratatui app w/ 11 overlays
```

For live LLM runs, export an API key (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.) and use `argo chat` or `argo session new`.

### Build from source

If you'd rather build instead of installing a binary:

```bash
git clone https://github.com/mark-hallam/Argo-Agent argo
cd argo
cargo build --workspace --release
./target/release/argo --version
```

## Optional features

Native voice STT (offline transcription via whisper.cpp):

```bash
cargo install --path mcp-servers/voice --features whisper-native
# adds ~50s to a fresh build (one-time C++/CMake compile of whisper.cpp)
# adds ~20 MB to the resulting binary
# download a ggml model — e.g. ggml-base.en.bin — from
# https://huggingface.co/ggerganov/whisper.cpp
export ARGO_WHISPER_MODEL=~/.argo/ggml-base.en.bin
```

## Documentation

- `docs/plan/argo-build-plan.md` — the master spec (v0.2)
- `docs/plan/argo-day-0-kickoff.md` — coding-agent kickoff pack
- `docs/plan/argo-vs-hermes-gap-analysis.md` — user-research-grounded design rationale
- `ARCHITECTURE.md` — condensed architecture overview
- `ROADMAP.md` — phase plan + per-PR map
- `CONTRIBUTING.md` — how to contribute (read this BEFORE opening a PR)

## License

Proprietary — all rights reserved. See `LICENSE`.
