# fix-routing-decision-tenant-scoping

**Owner**: codex.
**Depends on**: none (audit already pinned FINDINGS-ONLY in #2115).
**Authored against**: origin/main `0bf75c1a1` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-routing-decision-tenant-scoping`.

🟡 **P1 CLAIM-5 SUBSTRATE.** Wire `tenant_id` through the routing-decision substrate so per-tenant routing analytics, debugging, and cross-tenant safety checks are actually possible. Source: `eval-argo-routing-crate-surface` audit (#2115) — pinned FINDINGS-ONLY, fix-brief named.

## Why this is P1

Per gap manifest (`docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` line 84):
> `RoutingContext` / `RoutingDecision` / `routing_decision` table all elide `tenant_id` → fix-brief `fix-routing-decision-tenant-scoping` (🟡 P1) queued.

Today the routing layer has no concept of which tenant a routing decision belongs to. As `PersistingRouter` (today only wired via #2115 follow-up `fix-routing-decision-persist-wire`) lights up, tenant-scoped queries against `routing_decision` will be impossible to write without a schema change. Land the schema change now before the persist wire goes live, so the back-fill story is empty rather than retroactive.

## Scope

**In:**
- `crates/argo-routing/src/router.rs` — add `tenant_id: TenantId` to `RoutingContext<'_>`.
- `crates/argo-types/src/routing.rs` (or wherever `RoutingDecision` lives) — add `tenant_id: TenantId` field.
- `crates/argo-routing/src/persist.rs` — store `tenant_id` on insert; add tenant-scoped read helper `list_for_tenant(store, tenant_id) -> ArgoResult<Vec<RoutingDecision>>`.
- `crates/argo-store/src/migrations/` — additive migration that ALTERs `routing_decision` to add `tenant_id TEXT NOT NULL DEFAULT 'unknown'` then drops the default after backfill (or uses the `NULL`-then-backfill pattern per `docs/skycot/operator/migrations.md`).
- `crates/argo-routing/src/fixed.rs` — update test builders to pass tenant_id.
- All in-crate test builders.
- Un-ignore FINDINGS-ONLY pin tests added in #2115 (the audit shipped 2 `#[ignore]`'d tests; flip them live and assert tenant_id round-trips).
- 1 NEW cross-tenant negative test: tenant_b cannot read tenant_a's routing_decision rows via `list_for_tenant`.

**Out:**
- `crates/argo-agent-runner/src/turn_routing.rs:87` callsite (handled by sibling brief `fix-routing-decision-persist-wire`).
- Backfilling `tenant_id` on existing rows — leave default placeholder; production rows are not yet load-bearing.

## Acceptance

- `cargo test -p argo-routing` green including the un-ignored FINDINGS-ONLY tests + new cross-tenant negative.
- `cargo test -p argo-store` migration runs cleanly forward + idempotent on re-run.
- `cargo clippy -p argo-routing -p argo-store --all-targets -- -D warnings` clean.
- Migration slot does not collide (run `scripts/check-migration-slots.sh`).
- Gap-manifest row updated (a one-line edit to flip `(a)` from queued → SHIPPED).

## Branch + PR

- Branch: `codex-fix-routing-decision-tenant-scoping`
- PR title: `fix(routing): thread tenant_id through RoutingContext/RoutingDecision (fix-routing-decision-tenant-scoping)`

## Notes for the implementer

- ~150-250 LOC expected (mostly additive field-add + builders + migration + tests).
- Cite the gap manifest row + the FINDINGS-ONLY pin tests from #2115 verbatim in the PR body.
- Cross-tenant pin test follows the recipe pattern from #2168 / #2177 / #2184: build two TenantIds, insert one decision under each, assert `list_for_tenant(tenant_b)` returns 0 rows of tenant_a's, baseline same-tenant returns 1.
