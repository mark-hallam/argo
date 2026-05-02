# Argo Coding Agent Instructions

Argo is a commercial AI-agent platform.

The architecture is:

1. Core Platform
2. Vertical Agent Templates

The core platform is already substantial. Do not rebuild it unnecessarily.

Core platform includes:
- runtime
- tools
- provider routing
- memory
- skills
- audit
- gateway
- eval
- forge
- replay
- MCP
- RBAC
- marketplace

Vertical agents must be implemented through templates, not hardcoded
runtime forks.

Vertical-specific behavior belongs in:
- manifests
- workflow definitions
- prompt packs
- memory schemas
- tool permissions
- escalation policies
- evaluation cases
- UI/gateway metadata

Never place receptionist/support/PR/sales-specific logic into core
crates unless the abstraction is reusable across multiple verticals.

Priorities:
1. reliability
2. observability
3. repeatability
4. security
5. commercial usability
6. extensibility

Maintain:
- single-binary architecture
- Rust 1.75 compatibility
- <500 LOC per file
- additive migrations
- tests for every feature
- JSON output for CLI list/show/status commands

---

## Pre-existing core-platform invariants (still in force)

These were the v0.2 hard rules and remain unchanged:

Five non-negotiable invariants:
  1. Polyglot core (Rust core + JSON-RPC plugin ABI; plugins are subprocesses)
  2. Multi-tenant by construction (tenant_id plumbed through every primitive)
  3. Capability-based security with first-class RBAC
  4. Provider manifests, not adapters (TOML, not Rust)
  5. MCP-first integration (8 in-core tools; everything else MCP)

Hard rules — CI rejects PRs that violate any of these:
  - No file > 500 LOC
  - No fn > 80 LOC
  - No struct impl block > 200 LOC
  - No `unsafe` Rust outside marked FFI shims
  - No `unwrap`/`expect` outside `#[cfg(test)]`
  - No global mutable state. AgentDeps is the only injection point.
  - Every public Rust API has rustdoc with at least one example

When in doubt:
  - Subtract before adding. Idiot Index is solution complexity ÷ problem complexity.
  - Question the requirement. If something doesn't make sense, raise it; do not invent.
  - Ship the 80% solution. Working software beats the perfect solution that arrives late.

The master spec is `docs/plan/argo-build-plan.md` (v0.2) for the
core-platform layer. The vertical-template system is documented in
its own design doc once Phase 1 lands.

PR title format: "<phase> — <section> — <one-line summary>"

---

## Project rename — Argo → Skycot

The project will rename from Argo to **Skycot** (domain: skycot.com).
Continue using `argo` everywhere for now (binary, crates, env vars,
~/.argo/, argo.toml). The rename will land as one or two coordinated
PRs: a cheap repo-URL pass and an optional expensive technical-name
pass. Do not start renaming proactively.
