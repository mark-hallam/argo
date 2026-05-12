# Argo Coding Agent Instructions

Argo is a commercial AI-agent platform.

The architecture is:

1. Core Platform
2. Vertical Agent Templates

The core platform is already substantial. Do not rebuild it
unnecessarily.

Core platform includes: runtime, tools, provider routing, memory,
skills, audit, gateway, eval, forge, replay, MCP, RBAC, marketplace.

Vertical agents must be implemented through templates, not hardcoded
runtime forks. Vertical-specific behavior belongs in: manifests,
workflow definitions, prompt packs, memory schemas, tool permissions,
escalation policies, evaluation cases, UI/gateway metadata.

Never place receptionist/support/PR/sales-specific logic into core
crates unless the abstraction is reusable across multiple verticals.

## Priorities

1. reliability
2. observability
3. repeatability
4. security
5. commercial usability
6. extensibility

## Maintain

- single-binary architecture
- Rust 1.75 compatibility
- <500 LOC per file
- additive migrations
- tests for every feature
- JSON output for CLI list/show/status commands

## Core-platform invariants

Five non-negotiable invariants:

1. Polyglot core (Rust core + JSON-RPC plugin ABI; plugins are subprocesses)
2. Multi-tenant by construction (tenant_id plumbed through every primitive)
3. Capability-based security with first-class RBAC
4. Provider manifests, not adapters (TOML, not Rust)
5. MCP-first integration (8 in-core tools; everything else MCP)

CI rejects PRs that violate any of these:

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

Plan documents, in order of operative precedence:

- `docs/plan/argo-build-plan.md` (v0.2) — original architecture
  reference for the core-platform layer. Still valid for that scope,
  but no longer the operative phased plan.
- `docs/build-plan/BUILD_PLAN_V11.md` — operative phased plan for
  backend tracks (v1.1 production hardening through v2.2 demo
  verticals). Authoritative for backend scope; product-surface /
  frontend work moved to PRODUCT_ROADMAP.md as of 2026-05-11.
- `docs/build-plan/PRODUCT_ROADMAP.md` — operative plan for the
  post-MVP product-surface program (Tracks A–G: UX research →
  design language → wireframes → frontend implementation +
  parallel claim-4 backend fill-ins + claim-5 finish). Read this
  before picking up any UI / UX / persona / journey work.
- `docs/build-plan/BUILD_STATUS.md` — the live ledger (rows + state
  transitions, edited every tick).
- `docs/build-plan/MISSION_BACKLOG.md` — canonical source of
  un-shipped work, status-tagged for queue replenishment.
- `docs/skycot/ux-research/` — persona docs + journey maps. The
  design brief for all frontend implementation. Referenced from
  PRODUCT_ROADMAP.md Track A.

The vertical-template system is documented in its own design doc
once Phase 1 lands.

**Fresh agents: start at [`docs/build-plan/RESUME.md`](docs/build-plan/RESUME.md)** — a single-page primer pointing you at the
5 files to read (in order) to pick up the swarm. The
`docs/build-plan/BUILD_STATUS.md` header (first ~90 lines) is the
live ledger — operator amendments (queue thresholds, agent-family
availability, paused streams, active directives) live there and
override every other doc. Memory and template defaults are not
authoritative; the live ledger is.

PR title format: `<phase> — <section> — <one-line summary>`

## Project rename — Argo → Skycot

The project will rename from Argo to **Skycot** (domain: skycot.com).
Continue using `argo` everywhere for now (binary, crates, env vars,
`~/.argo/`, `argo.toml`). Do not start renaming proactively.

## Multi-agent swarm protocol

**The swarm is currently active.** Any session opened in this repo
is presumed to be a worker or conductor unless explicitly invoked
otherwise. Follow [`docs/build-plan/RESUME.md`](docs/build-plan/RESUME.md)
to pick up.

[`docs/build-plan/SWARM_PROTOCOL.md`](docs/build-plan/SWARM_PROTOCOL.md)
is the source of truth for: iteration sequence, hard rules,
verification, stop conditions, manager-worker mode, and graceful
exit. Re-read it on every iteration — the ledger on `origin/main` is
authoritative, not memory.

Companion docs:

- [`docs/build-plan/PROTOCOL.md`](docs/build-plan/PROTOCOL.md) — architecture and anti-patterns.
- [`docs/build-plan/PROTOCOL_LOOP_PROMPT.md`](docs/build-plan/PROTOCOL_LOOP_PROMPT.md) — conductor tick prompt.
- [`docs/build-plan/CONTRIBUTING_AGENT.md`](docs/build-plan/CONTRIBUTING_AGENT.md) — short worker companion.
- [`docs/build-plan/KNOWN_FLAKES.md`](docs/build-plan/KNOWN_FLAKES.md) — pre-existing test flakes.

Genuinely ad-hoc sessions (invoked outside the swarm context — e.g.
the operator running a one-off cleanup) can ignore the build-plan
layer; the core rules above still apply.
