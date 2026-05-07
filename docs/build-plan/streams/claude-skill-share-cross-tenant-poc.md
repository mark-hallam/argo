# claude-skill-share-cross-tenant-poc — cross-tenant skill sharing PoC

**Owner**: claude. **Depends on**: existing `org_chart` table (migration
`21__org_chart.sql`); existing skill versioning + `SkillStore`.

## Why

Mission claim 4 says "Cross-tenant sharing with provenance + trust." That
hasn't shipped yet. This brief is the proof-of-concept slice: a tenant can
mark a skill as `shared: true` and other tenants in the same `org_chart`
cluster can see + draw from it (read-only). No cross-cluster sharing, no
write-back, no signing — just a small, demonstrable wedge that proves the
data model.

## Scope

- Schema: extend the skill version table with `shared BOOLEAN NOT NULL DEFAULT 0`
  and `shared_with_cluster TEXT NULL` (nullable so the default is "share
  with own org_chart cluster"). Migration `38__skill_shared.sql`. Down
  migration drops the columns.
- API on `SkillStore`: `pub async fn list_shared_for_tenant(&self, tenant_id: &str) -> ArgoResult<Vec<SharedSkillRow>>`
  — returns skills marked `shared` whose owning tenant is in the same
  org_chart cluster as `tenant_id`.
- API: `pub async fn set_shared(&self, skill_id: &str, version: &str, shared: bool, cluster: Option<&str>) -> ArgoResult<()>`.
- **NEW** `crates/argo-store/src/skill_share.rs` (~220 LOC) holding the
  query helpers.
- Lookup path used by the agent runner is OUT OF SCOPE for this brief —
  this is data-model + reader API only.

## Approach

1. Confirm the canonical skill table name with
   `grep -rn "CREATE TABLE.*skill" crates/argo-store/migrations/`. Use the
   matching ALTER TABLE in the migration.
2. **NEW** `crates/argo-store/migrations/38__skill_shared.sql`:
   - UP: ALTER TABLE adding `shared`, `shared_with_cluster`; index on `shared`.
   - DOWN: drops the columns.
3. **NEW** `crates/argo-store/src/skill_share.rs` with the two helpers.
   Cluster lookup joins through `org_chart`.
4. Wire migration into `crates/argo-store/src/migrations/mod.rs` as v12 (per
   the existing v1..v11 pattern in that file). Coordinate with conductor
   if v12 is already claimed.

## Files you may touch

- `crates/argo-store/migrations/38__skill_shared.sql` (NEW).
- `crates/argo-store/src/migrations/v12.rs` (NEW, ≤ 80 LOC) wrapping the SQL.
- `crates/argo-store/src/migrations/mod.rs` — narrowly: 1 `mod v12;` + insertion in `all_migrations()`.
- `crates/argo-store/src/skill_share.rs` (NEW, ≤ 280 LOC).
- `crates/argo-store/src/lib.rs` — narrowly: 1 `pub mod skill_share;` line.
- `crates/argo-store/tests/skill_share.rs` (NEW, ≤ 250 LOC) OR equivalent
  inline `#[cfg(test)] mod` in skill_share.rs.

## Files you must NOT touch

- CLI surface — out of scope for the PoC.
- `crates/argo-agent-runner/` — runner integration is a follow-up.
- `crates/argo-skill/src/marketplace.rs` — different sharing model (OCI).
- Other migrations.

## Acceptance criteria

- [ ] Migration 38 applies cleanly UP and rolls back cleanly DOWN.
- [ ] `set_shared` toggles the flag and persists.
- [ ] `list_shared_for_tenant` returns only skills owned by tenants in the same org_chart cluster.
- [ ] Skills not marked shared are NEVER returned, regardless of cluster.
- [ ] Migration is idempotent (re-running UP is a no-op on already-applied state).
- [ ] 6+ tests: toggle, list-same-cluster, list-different-cluster (empty), unshared invisible, migration up/down, idempotence.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-share-cross-tenant-poc`
- PR title: `store: cross-tenant skill share PoC (claude-skill-share-cross-tenant-poc)`

## Notes

- `org_chart` schema lives in `21__org_chart.sql` — read it before writing the join.
- This brief deliberately stops short of provenance/signing/trust gates —
  those land as separate follow-ups (`claude-skill-share-provenance`, etc).
- Brief assumes "next free migration is 38+" per BUILD_PLAN_V11.
