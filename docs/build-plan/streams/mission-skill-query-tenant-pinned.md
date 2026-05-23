# mission-skill-query-tenant-pinned

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `3df52142a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-query-tenant-pinned`.

🔴 **P0 LATENT-LEAK (post-PR-#2092 sweep gap).** PR #2092 added a
`tenant_id` column to the `skill` table via migration v79 and swept
some SQL (e.g. `SkillStore::available_for_worker`) — but missed the
reader helpers in `skill_query.rs`. Three SELECTs return data
across tenants:

- `skill_query.rs:66` — `distinct_skill_verticals`:
  `SELECT DISTINCT vertical_id FROM skill WHERE vertical_id IS NOT
  NULL ORDER BY vertical_id`
- `skill_query.rs:80` — `search_direct`:
  `SELECT id, name, version, vertical_id, created_at FROM skill
  WHERE vertical_id = ? ORDER BY … LIMIT ?`
- `skill_query.rs:100` — `search_indirect`:
  JOIN against `session_skill` + `session` keyed on
  `sess.vertical_id = ?` only.

## Why

PR #2092 PR-body explicitly says it swept some SQL but not all of
the `skill`-table query surface. Found in
`eval-argo-store-tenant-bind-sweep-remaining-tables` §4.1 + §4.2.

Today's caller chain (`crates/argo-cli/src/handlers/skill_search.rs`)
is CLI-only, so the leak is latent. **The moment a route handler
wires `distinct_skill_verticals` or `search_by_vertical`, cross-tenant
skill enumeration is one HTTP GET away.** Same shape as PR #2116 §4
finding 1 (which shipped via #2146).

## Scope

- Add `tenant_id: TenantId` parameter to:
  - `skill_query::distinct_skill_verticals`
  - `skill_query::search_by_vertical` (both
    `search_direct` and `search_indirect` internal forms)
- WHERE clauses bind `tenant_id`:
  - `… AND tenant_id = ?` for `distinct_skill_verticals` (filter out
    `__migration_placeholder__` if needed)
  - `WHERE vertical_id = ? AND tenant_id = ?` for `search_direct`
  - `WHERE sess.vertical_id = ? AND sess.tenant_id = ?` for the
    indirect form (or `AND s.tenant_id = ?` if cleaner — verify at
    claim-time)
- Thread tenant through caller:
  `crates/argo-cli/src/handlers/skill_search.rs:43,61,75`.
- Cross-tenant pin test (two tenants seed skills with the same
  `vertical_id` value, query tenant_a, expect only tenant_a rows).
- Consider the `__migration_placeholder__` sentinel from PR #2092:
  decide whether legacy placeholder rows should be visible (probably
  not — gate behind a flag or filter them out by default).
- ~200-350 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/skill_query.rs`
- `crates/argo-store/tests/skill_query_tenant_bind_pin.rs` (new)
- `crates/argo-cli/src/handlers/skill_search.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] All three SELECTs bind `tenant_id` in SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
