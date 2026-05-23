# claude-spend-anomaly-tenant-scope-distinct

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-spend-anomaly-tenant-scope-distinct`.

🔴 **P1 CROSS-TENANT WORKER_ID EXISTENCE LEAK.** Source: evaluator round-13 SHARED.md handoff 2026-05-23T12:30:00Z.

> "🔴 spend_anomaly.rs SELECT DISTINCT worker_id FROM worker_budget — NO tenant scoping; lists every worker_id across every tenant (existence-leak; round-4 SQL-layer twin)."

The query enumerates `worker_id` across all tenants. A caller in tenant_b can read the result and harvest every worker_id in the workspace — same shape as the round-4 worker-handler existence-leak fixed in #2068, but at the SQL layer instead of the HTTP-response layer.

## Scope

1. Find query: `crates/argo-store/src/spend_anomaly.rs` (or sibling).
2. Add `tenant_id` parameter; add `WHERE tenant_id = ?` to the DISTINCT.
3. Sweep callers — thread tenant_id.
4. **Cross-tenant negative test**: bootstrap tenant_a with workers W1+W2 + tenant_b with workers W3+W4; call from tenant_b → assert ONLY [W3, W4] returned, NOT [W1, W2, W3, W4].

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-store/src/spend_anomaly.rs` — narrowly scoped.
- `crates/argo-store/src/spend_anomaly_tests.rs` (or sibling) — cross-tenant test.
- Caller sites (find via `rg`).
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The DISTINCT semantic — that's correct, just needs tenant scoping.
- Other anomaly-detection logic.

## Acceptance criteria

- [ ] Query binds `tenant_id`.
- [ ] All callsites updated.
- [ ] Cross-tenant negative test pinned.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `claude-spend-anomaly-tenant-scope-distinct`
- PR title: `fix(store): tenant-scope spend_anomaly DISTINCT worker_id (claude-spend-anomaly-tenant-scope-distinct)`

## Notes for the implementer

- ~80-150 LOC. Sibling brief to `claude-skill-runs-worker-budget-tenant-scope` — same shape, different surface.
- Cite round-13 verbatim + reference #2068 as the response-layer twin.
