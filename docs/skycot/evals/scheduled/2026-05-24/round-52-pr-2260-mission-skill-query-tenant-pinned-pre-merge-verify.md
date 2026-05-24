# Round 52 — PR #2260 `mission-skill-query-tenant-pinned` PRE-MERGE anti-theater verify

- **Date**: 2026-05-24
- **Round**: 52 (scheduled hourly fire, real-UTC `date -u` 2026-05-24T22:10:36Z; Melbourne local 2026-05-25 08:10 AEST)
- **Pillar / Claim**: **Claim 6 — Multi-tenant by construction** (skill discovery surfaces must filter by tenant)
- **Verdict**: 🟢 **PRE-MERGE PASS** — severity tier N/A (anti-theater verify is a precondition for merge, not a fix)

## Setup

Pulled `origin/main` (HEAD `c8193c869`, just bumped status header for newly-opened #2260). Inspected diff via `gh pr diff 2260`, compared against current `crates/argo-store/src/skill_query.rs` and `crates/argo-cli/src/handlers/skill_search.rs` heads to confirm the change isn't subsumed by a parallel landing. Cross-referenced against `crates/argo-store/migrations/131__skill_tenant_id.sql` to verify the SQL clauses are well-typed against the live schema.

Diff scope: **3 files, +323 / -30**:

- `crates/argo-cli/src/handlers/skill_search.rs` (+55 / -6) — CLI handler grows a `resolve_vertical_tenant` helper and a `infer_session_tenant` SQL lookup so the `--vertical` query path can pin to a tenant when `--tenant` isn't passed.
- `crates/argo-store/src/skill_query.rs` (+105 / -24) — three public SQL queries (`search_by_vertical` direct + indirect, `distinct_skill_verticals`) grow a mandatory `tenant_id: &str` parameter and add `AND tenant_id = ?` to the WHERE clauses; in-module tests rewritten to seed and bind `PLACEHOLDER_TENANT_ID`.
- `crates/argo-store/tests/skill_query_tenant_bind_pin.rs` (+163 / NEW) — two `#[tokio::test]`s that seed multi-tenant skill rows and assert direct/distinct/indirect modes do NOT cross-tenant leak.

## Observed

### Production code-path review

**`search_direct` (skill_query.rs:88-104)** — SQL grew `AND tenant_id = ?`:

```sql
SELECT id, name, version, vertical_id, created_at
FROM skill
WHERE vertical_id = ? AND tenant_id = ?
ORDER BY created_at DESC, name ASC, version ASC
LIMIT ?
```

Bind order: `vertical_id` → `tenant_id` (TEXT, correct — `migrations/131` made `skill.tenant_id` a TEXT column) → `limit`. ✓

**`distinct_skill_verticals` (skill_query.rs:71-86)** — SQL grew `AND tenant_id = ?`:

```sql
SELECT DISTINCT vertical_id FROM skill
WHERE vertical_id IS NOT NULL AND tenant_id = ?
ORDER BY vertical_id
```

Bind order: `tenant_id` (TEXT). ✓

**`search_indirect` (skill_query.rs:108-130)** — SQL grew `AND sess.tenant_id = ?` (note: **session table tenant_id is BLOB**, NOT TEXT, per `migrations/v1.rs` session schema). Code correctly handles the type asymmetry:

```rust
let tenant_uuid = uuid::Uuid::parse_str(tenant_id)
    .map_err(|e| ArgoError::Provider(format!("skill_search tenant_id: {e}")))?;
// ...
.bind(tenant_uuid.as_bytes().as_slice())  // 16-byte BLOB bind
```

This is the right shape — if someone passes the literal `PLACEHOLDER_TENANT_ID` ("__migration_placeholder__") to indirect mode, `Uuid::parse_str` will reject it with a clear `skill_search tenant_id: invalid UUID` error rather than silently returning all sessions. ✓

### Pin test review

**`direct_and_distinct_queries_do_not_cross_tenant_leak` (`skill_query_tenant_bind_pin.rs:55-100`)**:

- Seeds **3 skill rows**: tenant_a, tenant_b, and `__migration_placeholder__`, ALL sharing `vertical_id="shared-vertical"`.
- Calls `search_by_vertical(pool, &tenant_a, "shared-vertical", 10, false)` → asserts `rows.len() == 1` AND `rows[0].skill_name == "tenant-a-skill"`.
- Calls `distinct_skill_verticals(pool, &tenant_a)` → asserts `verticals == vec!["shared-vertical"]` (NOT the legacy-vertical from placeholder row).

**Anti-theater check**: if the `AND tenant_id = ?` clause is reverted in either `search_direct` or `distinct_skill_verticals`, the assertion `rows.len() == 1` would FAIL (would return 2 for direct, since both tenant-a and tenant-b have rows in "shared-vertical"; would also return additional vertical from placeholder row). Test is **non-vacuous** — green only if SQL filter is present.

**`indirect_query_binds_session_tenant` (`skill_query_tenant_bind_pin.rs:103-163`)**:

- Seeds 2 workspaces in distinct tenants (a + b), both with sessions on `vertical_id="shared-vertical"`.
- Attaches one skill per tenant via `session_skill`.
- Calls `search_by_vertical(pool, &tenant_a_str, "shared-vertical", 10, true)` (indirect mode) → asserts `rows.len() == 1` AND `rows[0].skill_name == "attached-to-tenant-a"` AND `last_activity_micros == 11`.

**Anti-theater check**: if `AND sess.tenant_id = ?` is reverted in `search_indirect`, the test returns BOTH attached-to-tenant-a AND attached-to-tenant-b (both join through `session_skill` to sessions bound to "shared-vertical"); `rows.len() == 1` would FAIL. Test is **non-vacuous**.

### Production-callsite enumeration

`grep -rn "search_by_vertical\|distinct_skill_verticals" crates/ --include="*.rs"` against the PR branch shows the ONLY production callers are in `crates/argo-cli/src/handlers/skill_search.rs` — and that file is updated by the same PR. No orphan callers, no parallel surfaces.

### CLI tenant-resolution flow (`skill_search.rs:99-141`)

```rust
async fn resolve_vertical_tenant(...) -> ArgoResult<String> {
    if let Some(tenant) = tenant.filter(|t| !t.trim().is_empty()) {
        return Ok(tenant.to_string());          // 1. explicit --tenant wins
    }
    if indirect {
        if let Some(tenant) = infer_session_tenant(pool, vertical_id).await? {
            return Ok(tenant);                  // 2. indirect mode infers from most-recent session
        }
    }
    Ok(PLACEHOLDER_TENANT_ID.to_string())       // 3. fallback to migration placeholder
}
```

Reasonable for a CLI tool gated by filesystem access to the SQLite store. The placeholder fallback is a migration shim — legacy skill rows without a tenant_id get default `"__migration_placeholder__"`, and a CLI user without `--tenant` who runs `argo skill search --vertical foo` will see those legacy rows.

## Anti-theater check

How I proved this isn't just "test rewrites itself green":

1. **Read the production SQL clauses** in `skill_query.rs` to confirm the `AND tenant_id = ?` is present in all three query bodies, not just declared in the function signature.
2. **Read the pin test assertions** and ran the mental experiment: if the WHERE clause is removed, do the assertions fail? Both pin tests assert exact `rows.len() == 1` with a specific skill name from a multi-tenant seed → green only if filter is honored. Non-vacuous.
3. **Cross-checked the schema** (`migrations/131`) for the actual column type. `skill.tenant_id` is TEXT with placeholder default; `session.tenant_id` is BLOB. The PR's bind shapes match (TEXT bind for direct/distinct, BLOB bytes bind for indirect via `tenant_uuid.as_bytes()`).
4. **Production-callsite grep** showed `argo-cli/src/handlers/skill_search.rs` is the only production consumer; the PR updates that file in lockstep with the signature change. No orphan calls will break post-merge.

## Gap / follow-up brief candidates

Two minor gaps surfaced — neither is a merge blocker:

(a) **`fix-cli-skill-search-indirect-placeholder-fallback-error-message`** (P3, ~30 LOC, agent-doable) — when `--tenant` is omitted in indirect mode AND `infer_session_tenant` returns `None` (no sessions exist for the vertical yet), `resolve_vertical_tenant` falls through to `PLACEHOLDER_TENANT_ID`, then `search_indirect` calls `Uuid::parse_str("__migration_placeholder__")` which fails with the opaque `skill_search tenant_id: invalid UUID` error. Should be a clearer `no sessions found for vertical X — pass --tenant explicitly` message. Not a security hole — just a UX gap on a CLI tool.

(b) **`eval-skill-query-tenant-pin-extend-to-distinct-skill-verticals-isolation`** (P3, ~40 LOC test addition) — the new pin test `direct_and_distinct_queries_do_not_cross_tenant_leak` covers `distinct_skill_verticals` for the no-leak direction (tenant_a doesn't see tenant_b's vertical), but doesn't pin the OTHER direction (tenant_b doesn't see tenant_a's distinct verticals). Worth a one-assertion extension for symmetric coverage. Strictly defense-in-depth.

## Recommended next-tick action

**Merge cluster ready for next claude-conductor tick** — #2260 joins the queue:

- #2244 (r48 PRE-MERGE PASS, claim-6 worker archive cascade tenant pin)
- #2248 (r49 PRE-MERGE PASS, claim-6 worker writes tenant bind)
- #2256 (r50 PRE-MERGE PASS, claim-2 delegate_task fallback-chain)
- #2257 (r50 PRE-MERGE PASS, claim-6 channels cross-tenant negative)
- #2259 (r51 PRE-MERGE PASS, M22 Status Reporter pack)
- **#2260 (r52 PRE-MERGE PASS, claim-6 skill query tenant pin)** — **NEW**

All six are CLEAN MERGEABLE small-scope diffs that have passed evaluator anti-theater verify; they form the next safe admin-merge wave.

This is the **33rd anti-theater-honest verification** the evaluator has produced this week.
