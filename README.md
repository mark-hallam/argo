<p align="center"><strong>Argo</strong> — the ship of agents</p>

# Argo

Argo is the agent framework that **a single engineer can fully audit on a weekend** and that **a 200-person company can deploy to production** without writing the multi-tenant layer themselves.

> **Status: production-ready as of PR #522 (April 2026).** Phases 0–7 + 4.5 + 6.5 + 7.5 all shipped across **636+ PRs** on `main`. The gateway exposes **133 endpoints** (91 GET + 42 POST) with universal `--json` + `?format=markdown`. Library + integration + observability tiers are feature-complete. v1.1 architectural cleanup also done: forge daemon split (#581), browser MCP stateful sessions (17 stateful tools + lifecycle + idle-reaper, #591/#593), voice MCP native STT via whisper-rs (opt-in via `--features whisper-native`, #636).
>
> **What's left** is decision-gated, not blocked: native TTS (waiting on `piper-rs` upstream to fix its `ort` 2.x build), OCI registry CI smoke (needs test creds), and optional cloud browser backends (Browserbase / BrowserUse — user-pays). See `ROADMAP.md` for the per-PR map.

## The five non-negotiable invariants

1. **Polyglot core.** Rust core + JSON-RPC plugin ABI. Plugins are subprocesses.
2. **Multi-tenant by construction.** `tenant_id` plumbed through every primitive.
3. **Capability-based security with first-class RBAC.** Every dispatch checks declared capabilities.
4. **Provider manifests, not adapters.** Quirks live in TOML, not Rust.
5. **MCP-first integration.** 8 in-core tools; everything else is an MCP server.

These five are the spec. Anything that violates them is a bug.

## Why not Hermes Agent?

Argo is a structurally different shape: Rust core, JSON-RPC plugin ABI, multi-tenant by construction, capability-based security, provider manifests instead of adapters, an MCP-first integration model — plus an external-judge adjudicator for skill creation, automated skill optimization (Argo Forge), time-travel session replay, and a workspace OCI marketplace. Roughly 5× smaller for ~95% of Hermes capability and addressing the 14 most-cited Hermes user pain points by design (token overhead, RBAC, multi-agent, adaptive timeouts, …). See `docs/plan/argo-vs-hermes-gap-analysis.md`.

## Install

One-liner install for macOS / Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.sh | bash
argo setup
```

The installer fetches a pre-built binary from the latest GitHub release (~12 MB) and drops it in `~/.local/bin/argo`. `argo setup` walks you through workspace init + provider key configuration.

Pin a specific version with `ARGO_VERSION=v0.1.0 ./install.sh` or change the destination with `ARGO_INSTALL_DIR=/opt/argo/bin`.

Windows users: download `argo-x86_64-pc-windows-msvc.zip` from the [Releases page](https://github.com/mark-hallam/Argo-Agent/releases) directly.

### Drive it offline (no API key required)

```bash
argo gateway start &           # bind 127.0.0.1:7400
curl http://127.0.0.1:7400/    # landing JSON + entry-point hints
curl http://127.0.0.1:7400/routes  # the full 133-endpoint catalog
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

MIT. See `LICENSE`.
