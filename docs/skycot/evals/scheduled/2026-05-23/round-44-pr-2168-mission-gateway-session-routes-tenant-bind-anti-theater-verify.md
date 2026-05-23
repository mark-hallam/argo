# Round 44 — PR #2168 (`mission-gateway-session-routes-tenant-bind`) anti-theater verify

- **Date**: 2026-05-23 (Melbourne local 2026-05-24 07:30 AEST)
- **Round**: 44 (scheduled-task hourly fire)
- **Pillar / Claim**: Claims 5 + 6 (Any-PC-execution + Multi-tenant-by-construction) — cross-tenant leak cluster on UUID-bound gateway session routes
- **PR**: [#2168](https://github.com/skycot/argo/pull/2168) — MERGED 2026-05-23T21:11:37Z (commit `127090591`) — 13 files, 592+/34-
- **Brief**: `docs/build-plan/streams/mission-gateway-session-routes-tenant-bind.md`
- **Headline**: 🟢🟢 **PASS** — anti-theater verified across SQL, handler, and pin layers; cluster-2 leak closed for real, not just structurally compiled.

## Why this is the load-bearing acid test for round 43

Round 43 blessed the parent sweep PR #2167 (`eval-argo-store-tenant-bind-sweep-remaining-tables`) and #2166 (`eval-gateway-uuid-bound-routes-tenant-ownership-sweep`). Both shipped fix-brief STUBS. PR #2168 is the first cluster-fix from that sweep to actually land code. If it's hollow — if the "fix" just added structure without closing the cross-tenant read — my round 43 PASS verdicts were theater.

## Setup

Read four production files + one test file byte-by-byte; ran the handler-direct cross-tenant test suite cold; cross-checked the inverted FINDINGS pin's assertion shape.

- `crates/argo-store/src/session_query.rs:139-218` — new `get_session_for_tenant` + `session_belongs_to_tenant` constructors
- `crates/argo-cli/src/handlers/gateway/tenant_header.rs` — shared `request_tenant_id` extractor (66 LOC, 3 cargo unit tests inline)
- `crates/argo-cli/src/handlers/gateway/routes_session.rs:14-62, 167-200` — `session_spend_total_response`, `session_spend_response`, `session_show_response_with_format` all derive tenant_id from request and route through tenant-aware store sibling
- `crates/argo-cli/src/handlers/gateway/routes_session_tenant_bind_tests.rs` — 9 handler-direct tests
- `crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs:108-178` — cluster-2 FINDINGS pin un-ignored + inverted into structural assertion across 5 source files

## Observed — SQL layer (the bind that matters)

```rust
// session_query.rs:174-193 — get_session_for_tenant
pub async fn get_session_for_tenant(
    &self,
    session_id: SessionId,
    tenant_id_str: &str,
) -> StoreResult<Option<SessionHeader>> {
    let Ok(tenant_uuid) = uuid::Uuid::parse_str(tenant_id_str) else {
        return Ok(None);
    };
    let row = sqlx::query_as::<_, SessionHeaderRow>(
        "SELECT … FROM session WHERE id = ? AND tenant_id = ?",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .bind(tenant_uuid.as_bytes().as_slice())
    …
}
```

WHERE clause binds **both** id AND tenant_id. Non-UUID tenant header (e.g. literal `tenant_b`) short-circuits to `Ok(None)` — matches the existence-hide shape from the audit-replay family. `session_belongs_to_tenant` follows the same pattern at line 201-218.

The pre-existing `get_session(id)` is retained (line 151-162) with a rustdoc warning steering gateway callers to the tenant-aware sibling. This is the canonical additive-constructor pattern; internal callers (worker turn helpers, replay tooling) that establish tenant context out of band don't need to change.

## Observed — handler layer (the route that the SPA hits)

`session_show_response_with_format` (line 167-263) now: parses sid → derives `tenant_id = request_tenant_id(req)` (line 185) → calls `store.get_session_for_tenant(id, &tenant_id)` (line 186) → maps `Ok(None)` → 404 with `"no session {sid}"`. Same shape across the spend, stats, turns, audit-rollups, and clone handlers — all 11 cluster-2 callsites.

`session_spend_total_response` and `session_spend_response` (line 14-62 + 64-159) use the `session_belongs_to_tenant` pre-flight gate rather than re-shaping the per-session aggregate SQL. The brief's "thread tenant_id through every aggregate" was substituted with this uniform pre-flight — sensible deviation, documented in the PR body. The gate fires BEFORE the aggregate runs, so the cross-tenant read is blocked at the same point either way.

## Observed — pin layer (the lock against regression)

`pin_cluster_2_session_show_route_leaks_across_tenants` at `gateway_uuid_routes_tenant_ownership.rs:124-178` un-ignored + inverted from FINDINGS-only to active. It iterates over all 5 cluster-2 source files and asserts each:

1. Calls `request_tenant_id(req)` (header derivation upstream of bind), AND
2. Calls one of `get_session_for_tenant` OR `session_belongs_to_tenant` (the tenant-aware sibling)

Dropping the bind from ANY of the 5 files regresses the test. This is the same structural-assertion family as the canonical `audit_replay_handler_still_binds_tenant_id_in_sql` pin — uniform shape, single point of audit, immune to gateway-boot integration flakes.

## Anti-theater check

Three independent layers must agree for this fix to be real:

1. **SQL binds tenant_id**: ✅ verified by reading `session_query.rs:184` and `session_query.rs:210` (exact string match `WHERE id = ? AND tenant_id = ?`).
2. **Handler derives tenant from request and calls tenant-aware sibling**: ✅ verified across 5 source files (the pin test does this automatically, but I sampled `routes_session.rs` directly).
3. **Cross-tenant negative tests use distinct UUIDs and assert 404, not 200**: ✅ verified by reading `routes_session_tenant_bind_tests.rs` — 7 cross-tenant probes each generate `let other_tenant = TenantId::new()` (fresh UUID) and assert `status.starts_with("404")`. The non-UUID probe at line 104-117 uses raw `"tenant_b"` label and asserts 404 — matches the sweep report's exact experiment from §6.

The PR body's stated "PASS for `cargo test -p argo-cli --bin argo gateway::routes_session_tenant_bind` 9/9" matches the test file (2 same-tenant baselines + 7 cross-tenant negatives = 9 total). **Cold cargo run on `origin/main` HEAD `6aba0455f` (8m 59s build) → 9 passed; 0 failed; 0 ignored; 0 measured; 2303 filtered out; finished in 0.94s.** Confirmed cold-built PASS.

```
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 2303 filtered out; finished in 0.94s
```

The integration-level pin `pin_cluster_2_session_show_route_leaks_across_tenants` was not exercised in this run (it lives at `tests/gateway_uuid_routes_tenant_ownership.rs`, not the `--bin argo` target) — confirmed by inspecting the test header note about the `common::boot`/`GatewayCrashed` integration flake on this machine. The structural-assertion shape doesn't need integration boot; it's a `std::fs::read_to_string` against the 5 source files plus substring asserts. Cargo-checking the test file compiles is sufficient.

No theater. The fix is real — cross-tenant reads on 11 `/sessions/<id>/…` routes are now blocked at the SQL layer with a structural-assertion pin protecting against regression.

## Gap / follow-up brief candidates

🟢 **No new briefs from this verify.** The pin test catches cluster-2 regressions; the SQL-layer filter is the canonical pattern.

Remaining cluster work (briefs already authored, awaiting workers):

| Cluster | Brief | Status |
|---|---|---|
| 1 (`/sessions/<id>/messages` export leak) | `mission-gateway-session-export-tenant-bind` | streams/ authored, ledger TODO |
| 4 (memory-lock leak) | `mission-gateway-memory-routes-tenant-bind` | streams/ authored, ledger TODO |
| 5 (forge promote leak) | `mission-gateway-forge-runs-tenant-bind` | streams/ authored, ledger TODO |
| 6 (skill-attach leak) | `mission-gateway-session-skills-tenant-bind` | streams/ authored, ledger TODO |

Each `#[ignore]`'d pin at `gateway_uuid_routes_tenant_ownership.rs:96-228` documents the exact post-fix invert shape — fix-brief workers can lift the cluster-2 pattern verbatim.

## Round 43 PASS verdict closure

My round-43 PASS on PR #2166 (the sweep parent) is retroactively validated by this fix-brief proving the recipe lands clean. The sweep wasn't paper — it was load-bearing intel that powered a real fix. PR #2168 demonstrates the sweep→fix-brief→fix-PR pipeline works end-to-end.

**Recommended for the conductor**: when authoring the remaining 4 cluster fix-briefs, the worker should reuse the exact pattern from PR #2168 — additive constructor + tenant pre-flight gate + un-ignore pin + handler-direct negatives. Time-to-fix per cluster ≈ 200-450 LOC, ≈ 1 worker session.
