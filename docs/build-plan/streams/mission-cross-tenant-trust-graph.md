# Mission load-bearing wire — cross-tenant trust graph primitive

**Owner**: claude. **Depends on**: `skill_quality_score` SHIPPED; `skill.shared` flag SHIPPED; `argo-skill` crate stable; `claude-skill-share-cross-tenant-poc` MERGED #1064; `claude-skill-trust-grade-display` SHIPPED.
**Authored against**: origin/main `a309aede` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cross-tenant-trust-graph`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 item #81 ("Build cross-tenant trust graph"). Cross-tenant skill sharing works (skill flagged `shared: true` is visible across tenants), and individual skill trust-grade displays work. But there's no GRAPH primitive: which tenants trust which sources, with what weight, decayed over what timeframe. Mission Claim 4 ("skills shared and learned") needs trust to compose — Tenant A trusts Tenant B's skills heavily, Tenant B trusts Tenant C lightly, so Tenant A inherits a moderate trust signal for C's skills.

## Scope

Add a `TrustGraph` primitive that materializes pairwise trust scores from observed signals:

```rust
pub struct TrustEdge {
    pub from_tenant: TenantId,
    pub to_tenant: TenantId,
    pub weight: f32,           // 0.0–1.0 — composed from quality scores + adoption rate
    pub observation_count: u32,
    pub last_updated_at: DateTime<Utc>,
}

pub struct TrustGraph {
    edges: HashMap<(TenantId, TenantId), TrustEdge>,
}

impl TrustGraph {
    pub fn from_observations(obs: &[TrustObservation]) -> Self;
    pub fn direct_trust(&self, from: &TenantId, to: &TenantId) -> Option<f32>;
    pub fn composed_trust(&self, from: &TenantId, to: &TenantId, max_hops: usize) -> Option<f32>;
    pub fn neighbors(&self, tenant: &TenantId) -> Vec<&TrustEdge>;
}

pub struct TrustObservation {
    pub source_tenant: TenantId,        // who shipped the skill
    pub consumer_tenant: TenantId,      // who adopted it
    pub skill_id: SkillId,
    pub quality_score: f32,             // observed quality on the consumer side
    pub observed_at: DateTime<Utc>,
}
```

**Composition semantics** (`composed_trust`):
- Hop 1 = direct edge weight.
- Hop 2+ = product of edge weights along the path, choosing the maximum across alternative paths (BFS to `max_hops`, default 3).
- Trust does NOT compose past 3 hops (signal decays too far to be meaningful).

NEW persisted table `trust_observation` (immutable append-only); `TrustGraph::from_observations` materializes the graph from a query window (default last 90 days).

CLI: `argo trust graph --from <tenant>` prints neighbors with weights; `argo trust composed --from <a> --to <c>` prints the composed score + the path taken.

This is NOT in scope: trust enforcement (downstream consumers — separate brief if needed for `auto-install-shared-skill` gate); cross-tenant invitation/handshake (still operator-driven); negative trust (downvote signals); decay function tuning (linear-time-decay only for v1).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__trust_observation.sql` (NEW) — `CREATE TABLE trust_observation (...)` per columns above. Implementer MUST verify next free slot via `grep -E "v4[5-9]|v5[0-9]" docs/build-plan/BUILD_STATUS.md` to avoid collision with in-flight briefs (v45 = #1271, mission-escalation-on-no-reply targets v50/51, mission-persona-evolution targets ≥v52).
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned migration matching SQL.
- `crates/argo-store/src/trust_observation.rs` (NEW, ≤ 250 LOC) — `record_observation`, `list_observations_in_window`. Per-tenant scoped lookups.
- `crates/argo-skill/src/trust_graph.rs` (NEW, ≤ 400 LOC) — `TrustGraph`, `TrustEdge`, `TrustObservation`, BFS composition. Pure-Rust, no DB dependency.
- `crates/argo-skill/src/trust_graph_tests.rs` (NEW, sibling) — 10 unit tests: direct-trust; composed-2hop; composed-3hop-cap; choose-max-path; isolated-tenant-returns-None; observation-aggregation; cycle-handled; neighbors-returns-all; weight-clamps-to-1.0; empty-graph-no-panic.
- `crates/argo-cli/src/handlers/trust_graph.rs` (NEW, ≤ 200 LOC) — `argo trust graph` + `argo trust composed` CLI handlers.
- `crates/argo-cli/tests/trust_graph_smoke.rs` (NEW, ≤ 150 LOC) — integration smoke: seed 3 tenants A→B→C with quality observations, assert composed_trust(A→C) > 0 and < direct_trust(A→B).

## Files you must NOT touch

- `crates/argo-skill/src/quality.rs` — QualityScore is upstream signal; reuse only
- `crates/argo-skill/src/sign.rs` — signing layer is upstream; orthogonal to trust graph
- Any non-store/non-skill/non-cli crate
- Any in-flight migration slot (verify before claiming)

## Acceptance criteria

- [ ] `TrustGraph`, `TrustEdge`, `TrustObservation` types exist
- [ ] `from_observations` aggregates per-edge: weight = mean(quality_score) clamped 0.0..=1.0, weighted by recency (linear decay over 90 days)
- [ ] `direct_trust(a, b)` returns edge weight or None
- [ ] `composed_trust(a, c, max_hops=3)` BFS-explores all paths; returns max(product of edge weights along path)
- [ ] Composition caps at `max_hops` (default 3) to prevent runaway
- [ ] `neighbors(a)` returns all outgoing edges from tenant a
- [ ] Cycles in the graph do not deadlock the BFS
- [ ] Migration adds `trust_observation` table idempotently
- [ ] `argo trust graph --from <t>` JSON-prints neighbors
- [ ] `argo trust composed --from <a> --to <c>` prints score + path
- [ ] 10 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-skill -p argo-cli` clean
- [ ] `cargo clippy -p argo-store -p argo-skill -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-skill -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cross-tenant-trust-graph`
- PR title: `skill: cross-tenant trust graph primitive (mission-cross-tenant-trust-graph)`

## Notes for the implementer

- The 3-hop cap is the brief's default. Operators arguing for 5+ hops should file a follow-up brief — trust signal at 4+ hops is genuinely lossy and 3 hops is the right MVP.
- Composition picks **max** across paths, not sum or average. The semantic is "I trust C because the BEST path to C through someone I trust gives me a good signal" — not "I trust C because many paths exist."
- Linear time-decay over 90 days is v1. Exponential decay is a follow-up brief.
- Weight clamping is load-bearing: a single rogue quality score >1.0 (shouldn't happen but defense-in-depth) should not poison the entire edge.
- `from_observations` is intentionally pure — no DB IO. The CLI layer queries `list_observations_in_window` and passes the Vec to the constructor. This keeps the graph testable without a DB fixture.
