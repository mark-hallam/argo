# Round 47 — PR #2177 `mission-skill-share-tenant-pinned` POST-MERGE anti-theater verify

**Date**: 2026-05-24 (UTC) · **Round**: 47 · **Pillar**: Multi-tenant by construction (Claim 6) · cross-tenant mutation pin · MARKETPLACE INSTALL EXPLOIT VECTOR
**Verdict**: 🟢🟢 **POST-MERGE PASS — round-43→46 P0 ageing watch CLOSED**

## Setup

- PR #2177 MERGED 2026-05-24T07:11:14Z by conductor (manual codex tick) as `0d35dc80a` (sha) after admin Tier-1 diff-shape review.
- Round-43 elevated this to P0 (substrate finding from PR #2167 sweep). Round-44, 45, 46 successively re-flagged "still unclaimed Xh+" — by round-46 was 7h+ ageing. Operator override 2026-05-24 cleared the queue for codex worker to claim + ship.
- Production fix: `crates/argo-store/src/skill_share.rs` adds new `set_shared_for_tenant(store, tenant_id, name, version, shared, cluster)` with `WHERE name = ? AND version = ? AND tenant_id = ?` (the missing bind). Legacy `set_shared` becomes wrapper binding `__migration_placeholder__` so legacy callsites cannot mutate real tenant rows.
- Caller updated: `crates/argo-cli/src/handlers/gateway/marketplace.rs:252-258` (`POST /marketplace/<id>/install`) now threads `&auth.tenant_id` as the new first arg.
- New test file `crates/argo-store/tests/skill_share_tenant_bind_pin.rs` (+107 lines, 3 tests).

## Observed

Cold `cargo test -p argo-store --test skill_share_tenant_bind_pin` against `origin/main` HEAD (fresh `CARGO_TARGET_DIR`):

```
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
```

Tests verified by source-read:
1. `tenant_scoped_share_does_not_mutate_another_tenant_skill` — inserts `(tenant_a, payroll, 1.0.0)`, calls `set_shared_for_tenant(tenant_b, payroll, 1.0.0, true, b-cluster)`, asserts tenant_a's row stays `(shared=0, cluster=None)`. **This is the exact PR #2167 round-43-discovered cross-tenant marketplace-install exploit vector pinned shut.**
2. `tenant_scoped_share_updates_matching_tenant_skill` — same-tenant baseline (no false negative on legitimate share).
3. `legacy_wrapper_only_targets_placeholder_tenant_rows` — wrapper safety: legacy `set_shared` flips legacy-placeholder rows but cannot reach real `tenant-a` rows (verified the placeholder discriminator is enforced).

## Anti-theater check

- Production SQL inspected at `skill_share.rs:115-130` — the literal SQL string is `"UPDATE skill SET shared = ?, shared_with_cluster = ? WHERE name = ? AND version = ? AND tenant_id = ?"` with `.bind(tenant_id)` as the fourth bind. Not a wrapper or runtime check that could be bypassed — the missing predicate is **in the SQL itself**. No way for the query to mutate a non-matching tenant_id row.
- Caller change at `marketplace.rs:252-258` threads `&auth.tenant_id` (authenticated handler) into the call site. Cannot be spoofed by request body.
- Test asserts `(0, None)` for the victim row — meaningful negative assertion that would fail loudly if the predicate were ever dropped.
- The legacy `set_shared` wrapper binding `__migration_placeholder__` is the load-bearing design choice: it preserves API surface for hypothetical legacy callsites without leaving them as cross-tenant mutation vectors. Test 3 pins that the wrapper cannot reach real tenant rows.

## Gap / follow-up

NONE. Round-43→46 P0 ageing watch closed cleanly with this merge. **24th anti-theater-honest substrate contract on main this week** (counting r45 #2131+#2169 as 22-23rd, then this #2177).

The CROSS-TENANT MUTATION VECTOR in marketplace install is shut at the production SQL level + test-pinned via memory-store integration test.

## Brief candidates surfaced this round

None for this scenario. Recommend conductor mark ledger row `mission-skill-share-tenant-pinned` → MERGED-FINDINGS-CLOSED (currently MERGED; no separate findings-only fix-by clock to start because the test landed in the SAME PR as the fix).
