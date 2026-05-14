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
- additive migrations (see `docs/skycot/operator/migrations.md`)
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

## v1.0 strategic decisions (operator-locked 2026-05-14)

These are not architectural preferences; they are operator decisions that scope v1.0. Read before authoring briefs or designing implementations.

- **macOS is the v1 demo path for computer-use.** `argo-executor-computer-use-macos` is the only OS-native executor v1 hardens. E2B, Linux, and Windows executors are DEFERRED-v1.x — they stay shipped, just not v1-polished. See `streams/claude-defer-non-macos-computer-use.md`. Demo recording `docs/skycot/demos/v1-mission-mvp.mp4` is macOS.
- **Enterprise focus, not consumer.** SKYCOT serves enterprise customers. MCP connector priority and authoring decisions reflect this. The 4 consumer/redundant connector stubs (signal, telegram, mailgun, resend) are being deleted; the 3 enterprise stubs (apollo, zoominfo, zoom) stay as DEFERRED-v1.x placeholders. When in doubt about a connector, ask "is this enterprise?"
- **Type canonicalization is in progress.** Phase A audit found 11+ duplicate `pub struct/enum` definitions across crates (worst: `AuditRow` in 4 crates, real data-integrity risk). 4 canonicalization briefs + 1 CI-lint brief are authored; do not introduce new parallel `pub` types until the canonical home is established. Re-export from `argo-types` is the preferred pattern when in doubt.
- **Skill-library worker load is the v1 SHIP-BLOCKER.** Phase A.5 audit found that `argo-agent-runner/src/turn.rs` never calls `LayeredLookup::with_library` — workers do NOT load marketplace skills at runtime. MISSION.md success scenario #2 silently fails. Brief: `streams/claude-skill-library-worker-load-wire.md` (~150 LOC). Ship this first.
- **Migration slot collisions are P0 operational risk.** Phase D audit found 11 slot-number collisions in `crates/argo-store/migrations/` (slots 19, 22, 23, 24, 26, 27, 28, 30, 45, 51, 75 — some 3-way). sqlx runner may process duplicates silently in filesystem order. Brief: `streams/claude-migration-slot-dedup-and-validator.md` (~500 LOC). Ship in the first wave alongside the SHIP-BLOCKER.

The full audit reports live in `docs/skycot/audit/` (5 phases, all complete 2026-05-14):
- `2026-05-14-phase-A-crate-inventory.md` — 88 crates × 6 claims mapping
- `2026-05-14-phase-A.5-skills-loop.md` — claim-4 edge trace (found SHIP-BLOCKER)
- `2026-05-14-phase-B-per-crate-deep-dive.md` — top 20 by LOC (verdict: production-ready, 1 orphan)
- `2026-05-14-phase-C-mission-claim-coverage.md` — E2E coverage per claim
- `2026-05-14-phase-D-cross-cutting-drift.md` — 11 migration collisions + 19 new parallel names + 236 TOML violations

**Status 2026-05-15:** 12 of the 22 remediation briefs already MERGED by codex/claude workers (Track M Phase 1 mostly done; type canonicalization round 1 at 2/4). 🔴 SHIP-BLOCKER + P0 migration-slot still TODO — both CLAUDE-PREFERRED, held for the fresh session 2026-05-19. See `docs/build-plan/RESUME.md` "Right-now snapshot" for the per-PR breakdown + remaining priority order.

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
- [`docs/skycot/DRIFT_PREVENTION.md`](docs/skycot/DRIFT_PREVENTION.md) — drift patterns, brief pre-flight checklist, audit cadence, unfucking severity ladder.

Genuinely ad-hoc sessions (invoked outside the swarm context — e.g.
the operator running a one-off cleanup) can ignore the build-plan
layer; the core rules above still apply.

## Drift prevention (conductor + worker)

The audit chain on 2026-05-13/14 surfaced a ~24% intent-drift rate in
conductor-authored briefs (new parallel surfaces alongside ones that
already shipped) plus systemic dead-code drift (subsystems landed
without callers, 17 connector crates with zero workspace consumers,
duplicate `SessionId`/`JudgeVerdict`/`Role` types across crates). The
full playbook is [`docs/skycot/DRIFT_PREVENTION.md`](docs/skycot/DRIFT_PREVENTION.md).
Load-bearing rules, inline so they cannot be missed:

- **Grep before write.** Before declaring any new pub struct, enum, or
  type alias in a brief, run `rg 'pub (struct|enum|type) <Name>' crates/`.
  If a same-named or near-same-named surface exists, the brief either
  extends it, supersedes it, or carries a one-line justification.
- **Wire or defer.** Every new public surface in a brief names its
  consumer. If the consumer doesn't exist yet, the brief either
  includes the wire-up or is paired with a follow-up brief in the same
  batch. "Land scaffold now, wire later" is the failure mode.
- **No parallel abstractions.** Two structs with the same name and
  overlapping responsibilities in different crates is a bug, not a
  feature. Past examples that became P1s: `McpConnectionPool` (in
  `argo-agent-runner`) parallel to canonical `McpClientPool` (in
  `argo-mcp`) without per-tenant keying → cross-tenant credential
  leak.
- **Freshness gate.** Briefs declare `Authored against: origin/main
  <SHA> on <YYYY-MM-DD>` within 24h of authoring, plus
  `Validate before claim: bash scripts/check-brief-freshness.sh
  <brief-id>`. Stale briefs are refused at claim time.
- **Recovery briefs re-validate intent.** Before authoring
  `claude-rebase-pr-<N>`, confirm the underlying intent is still
  load-bearing on current `origin/main`. If the bug or feature has
  already been fixed by another PR, close the original PR with
  STALE-FIXED instead.
- **Ledger drift sweep every tick.** Conductor runs
  `scripts/find-stale-claim-drift.py --quiet` every tick. Zero drift
  rows is the steady state.
- **Lint briefs before commit.** `scripts/lint-brief.sh <brief-id>`
  validates a brief's freshness header, `Validate before claim:`
  line, parallel-surface declarations against `crates/`, and
  parallel-name brief-id collisions against `streams/` + the ledger.
  A pre-commit hook (install via `scripts/install-brief-hooks.sh`)
  runs the lint on every staged brief in WARN mode; flip to enforce
  with `git config skycot.brief-lint enforce`.
