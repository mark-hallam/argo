<p align="center"><strong>Argo</strong> — the ship of agents</p>

# Argo

Argo is the agent framework that **a single engineer can fully audit on a weekend** and that **a 200-person company can deploy to production** without writing the multi-tenant layer themselves.

> **Status:** Phase 0 — skeleton. Walking-skeleton ship target: end of Week 3.
> See `docs/plan/argo-build-plan.md` for the full master spec and `ROADMAP.md` for what's shipping next.

## The five non-negotiable invariants

1. **Polyglot core.** Rust core + JSON-RPC plugin ABI. Plugins are subprocesses.
2. **Multi-tenant by construction.** `tenant_id` plumbed through every primitive.
3. **Capability-based security with first-class RBAC.** Every dispatch checks declared capabilities.
4. **Provider manifests, not adapters.** Quirks live in TOML, not Rust.
5. **MCP-first integration.** 8 in-core tools; everything else is an MCP server.

These five are the spec. Anything that violates them is a bug.

## Why not Hermes Agent?

Argo is a structurally different shape: Rust core, JSON-RPC plugin ABI, multi-tenant by construction, capability-based security, provider manifests instead of adapters, an MCP-first integration model — plus an external-judge adjudicator for skill creation, automated skill optimization (Argo Forge), time-travel session replay, and a workspace OCI marketplace. Roughly 5× smaller for ~95% of Hermes capability and addressing the 14 most-cited Hermes user pain points by design (token overhead, RBAC, multi-agent, adaptive timeouts, …). See `docs/plan/argo-vs-hermes-gap-analysis.md`.

## Quick start (once v1.0 ships)

```bash
curl -fsSL https://argo-agent.sh/install | bash
argo
```

Today, you build from source:

```bash
git clone https://github.com/mark-hallam/Argo-Agent argo
cd argo
cargo build --workspace
cargo test --workspace
./target/debug/argo --version
```

## Documentation

- `docs/plan/argo-build-plan.md` — the master spec (v0.2)
- `docs/plan/argo-day-0-kickoff.md` — coding-agent kickoff pack
- `docs/plan/argo-vs-hermes-gap-analysis.md` — user-research-grounded design rationale
- `ARCHITECTURE.md` — condensed architecture overview
- `CONTRIBUTING.md` — how to contribute (read this BEFORE opening a PR)

## License

MIT. See `LICENSE`.
