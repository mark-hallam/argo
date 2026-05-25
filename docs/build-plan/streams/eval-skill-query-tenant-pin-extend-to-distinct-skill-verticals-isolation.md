# Stream eval-skill-query-tenant-pin-extend-to-distinct-skill-verticals-isolation — extend `#2260` tenant-pin to symmetric coverage on `distinct_skill_verticals`

**Owner**: claude (Rust test extension; ~40 LOC).
**Depends on**: `#2260` MERGED (which added `crates/argo-store/tests/skill_query_tenant_bind_pin.rs`).
**Authored against**: origin/main `234ffd9f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-skill-query-tenant-pin-extend-to-distinct-skill-verticals-isolation`.

🟢 **P3 defense-in-depth coverage.** Source: round-52 evaluator finding (l) carried in r53 SHARED.md handoff.

## Scope

The PR #2260 tenant-pin test (`direct_and_distinct_queries_do_not_cross_tenant_leak`) covers the `distinct_skill_verticals` no-leak property in the **tenant_a → tenant_b** direction (tenant_a's query doesn't see tenant_b's verticals). The symmetric assertion (tenant_b's query doesn't see tenant_a's verticals) is missing. The fix is a one-assertion extension.

Why bother: defense-in-depth + future-proofs against asymmetric tenant-binding regressions. A reviewer who flips the WHERE clause in only one direction could pass the existing test today; the symmetric assertion makes that class of regression impossible.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`crates/argo-store/tests/skill_query_tenant_bind_pin.rs`]

## Files you may touch

- `crates/argo-store/tests/skill_query_tenant_bind_pin.rs` — extend `direct_and_distinct_queries_do_not_cross_tenant_leak` (added in #2260) with the symmetric tenant_b → tenant_a assertion block. Roughly: after the existing tenant_a-side assertions, repeat the `distinct_skill_verticals(pool, tenant_b)` call and assert the returned set does NOT contain any of tenant_a's seeded verticals.

## Tests

- The extended test itself (~20-30 LOC additional). The test should fail if either direction's tenant-bind is missing.

## Definition of done

- `cargo test -p argo-store --test skill_query_tenant_bind_pin direct_and_distinct_queries_do_not_cross_tenant_leak` green.
- The extension is non-vacuous: deliberately remove `AND tenant_id = ?` from the `distinct_skill_verticals` query (one-line edit), confirm the test FAILS, then restore — verify in the PR body that you did this check.
- PR body cites the round-52 finding (l) + this brief.

Notes: small, mechanical, low-risk. Good first-claim for a worker getting back into the swarm.
