# Mission load-bearing wire — RoutingEngine SkillMatch + RoleMatch (Tier M4) — REWRITTEN

**Owner**: claude. **Depends on**: `mvp-worker-skills-table` MERGED (provides `IdentityStore::workers_with_skill`); `mvp-worker-trust-grade-column` MERGED #1273 (provides `worker.trust_grade` + `last_used_at`); `TargetSelector::SkillMatch(String)` / `RoleMatch(String)` enum variants SHIPPED (verified at `crates/argo-orchestrator/src/routing.rs:166-181`).
**Authored against**: origin/main `5e1ef545` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-routing-skillmatch-rolematch`.

> **REWRITTEN 2026-05-11**: original brief was BLOCKED because it cited fields that don't exist (`Handoff.required_skill`, `WorkerRegistry::list_for_tenant`, `worker_skill` table). The prereqs are now landing: #1273 SHIPPED (trust_grade), `mvp-worker-skills-table` provides the table + lookup. This rewrite uses the actual shipped APIs.
>
> SOURCE: MISSION_BACKLOG.md Section 3 Claim 2 #51 — "Implement RoutingEngine::route for SkillMatch and RoleMatch" (currently PARTIAL — handoff-routing-rules #883 SHIPPED but skill_match/role_match return None per the inline doc comments at routing.rs:176-178).

## Scope

Implement `RoutingEngine::route` so the existing `TargetSelector::SkillMatch(String)` and `TargetSelector::RoleMatch(String)` variants resolve to a concrete `WorkerId` instead of returning `None`.

**Algorithm**:

1. **SkillMatch(skill_name)**:
   - Look up `skill_id` from `skill_name` via `SkillStore::find_by_name`.
   - Call `IdentityStore::workers_with_skill(tenant, skill_id)` → `Vec<WorkerId>`.
   - For each worker, fetch `(trust_grade, last_used_at)` via `IdentityStore::list_workers`.
   - Rank: `trust_grade DESC, last_used_at DESC` (tie-broken by worker_id for determinism).
   - Return the top match's `WorkerId` wrapped in `EscalationStep`.

2. **RoleMatch(role_name)**:
   - Requires a `role_name` column on `worker`. **PRECURSOR REQUIRED**: a small migration adding `role_name TEXT NULL` to the worker table. **Implementer authors this as part of the brief** (single new migration + setter + parameter on worker_onboard CLI).
   - Once present: `IdentityStore::workers_with_role(tenant, role_name)` → same rank-and-pick as above.
   - For v1: if `role_name` is universally NULL (no operators have populated it yet), return None. Empty result is not an error.

3. **NoMatch / empty list**: return `None`. Existing escalation handler picks up.

**Wiring**: extend `RoutingContext` to carry `&dyn TargetResolver` where `TargetResolver` is a new narrow trait in `argo-orchestrator`:

```rust
pub trait TargetResolver: Send + Sync {
    async fn resolve_skill_match(&self, tenant: &TenantId, skill_name: &str)
        -> ArgoResult<Option<WorkerId>>;
    async fn resolve_role_match(&self, tenant: &TenantId, role_name: &str)
        -> ArgoResult<Option<WorkerId>>;
}
```

`argo-cli` impls this on a thin struct that holds `Arc<IdentityStore>` + `Arc<SkillStore>`. Keeps `argo-orchestrator` free of cross-crate identity/skill deps.

This is NOT in scope: changing the `RoutingRule` schema; multi-step escalation chains (separate brief); persisting routing decisions to audit (orthogonal); skill-version-specific routing.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__worker_role_name.sql` (NEW) — `ALTER TABLE worker ADD COLUMN role_name TEXT NULL;`. **Verify next free slot** vs in-flight briefs.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned migration.
- `crates/argo-identity/src/store.rs` (or sibling) — narrowly scoped: ADD `set_worker_role_name(worker_id, role_name)`, `workers_with_role(tenant, role_name)`, and read role_name in `worker_from_row`. Verify file LOC; place in sibling if at-risk.
- `crates/argo-cli/src/handlers/worker_onboard.rs` — narrowly scoped: ADD optional `--role-name <s>` flag pass-through.
- `crates/argo-orchestrator/src/routing.rs` — narrowly scoped: extend `RoutingContext` with `target_resolver: &dyn TargetResolver` (Option<&...> or non-optional — implementer decides for back-compat with existing tests). Implement SkillMatch/RoleMatch arms in `route()` calling into the resolver. NO change to `TargetSelector` variants. Keep file under 500 LOC; split to `routing_resolve.rs` if needed.
- `crates/argo-orchestrator/src/routing_resolve.rs` (NEW, if split, ≤ 200 LOC) — `TargetResolver` trait + helper impl skeletons.
- `crates/argo-orchestrator/src/routing_tests.rs` — narrowly scoped: ADD 5 NEW unit tests: skill-match-returns-highest-grade; role-match-returns-highest-grade; both-empty-returns-none; tied-grade-tied-recency-deterministic-by-id; resolver-error-falls-through-to-none.
- `crates/argo-cli/src/handlers/...` — NEW thin `TargetResolver` impl (≤ 100 LOC).
- `crates/argo-cli/tests/handoff_routing_e2e.rs` (NEW, ≤ 250 LOC) — 1 integration test: seed 2 workers with skill X (different trust grades), open handoff with SkillMatch, assert higher-grade worker wins.

## Files you must NOT touch

- `crates/argo-orchestrator/src/auction*.rs` / `walker.rs` — orthogonal orchestrator pieces
- `crates/argo-skill/src/store.rs` `find_by_name` — assumed to exist; reuse only
- `crates/argo-identity/src/store.rs` `list_workers` + `workers_with_skill` — reuse only
- Any non-orchestrator/non-identity/non-cli crate

## Acceptance criteria

- [ ] Migration adds `role_name TEXT NULL` column to worker table
- [ ] `IdentityStore::set_worker_role_name` + `workers_with_role` exist
- [ ] `TargetResolver` trait in argo-orchestrator with two async methods
- [ ] `RoutingEngine::route` calls the resolver for SkillMatch + RoleMatch
- [ ] Ranking is `trust_grade DESC, last_used_at DESC, worker_id ASC` for determinism
- [ ] Tied grades + tied recency → deterministic by worker_id
- [ ] Resolver error → route returns None (don't crash routing)
- [ ] `argo worker onboard --role-name <s>` works
- [ ] 5 new unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-identity -p argo-orchestrator -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] All existing routing tests still pass byte-identical
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-routing-skillmatch-rolematch`
- PR title: `orchestrator: RoutingEngine SkillMatch + RoleMatch via TargetResolver (mission-routing-skillmatch-rolematch)`

## Notes for the implementer

- The original brief's premise was wrong about `Handoff.required_skill`/`required_role` — those fields don't exist. The new design uses the existing `TargetSelector::SkillMatch(String)`/`RoleMatch(String)` enum variants which DO exist (routing.rs:166), and resolves them via the new TargetResolver trait.
- The TargetResolver indirection is load-bearing: it keeps argo-orchestrator free of argo-identity/argo-skill deps. The thin impl lives in argo-cli where both crates are already in the dep graph.
- Migration slot: check v45/v46/v47 status before claiming (other in-flight briefs use these).
- The `role_name` column is a simple TEXT NULL — operators populate it via the new CLI flag. Empty result is normal in v1 because most existing workers won't have it set.
- For the integration test: use the existing test harness (other handoff tests under `crates/argo-cli/tests/` show the pattern).
