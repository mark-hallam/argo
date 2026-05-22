# mission-workplace-knowledge-substrate — new crate `argo-workplace-knowledge` + 2 tables for tenant-scoped wiki

**Owner**: claude.
**Authored against**: origin/main `90fb64111` on `2026-05-22` (re-validated; original auth `c613d922` 2026-05-21).
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-workplace-knowledge-substrate`.
**Priority**: 🚨 **P0 — OPERATOR-LOCKED 2026-05-22** — buyer-trial substrate. Mission claim 4 ("Skills shared + learned") cannot demo end-to-end without this. Gates 5 dependents (2 BLOCKED + 3 TODO). Auditor surfaced as P0 across 5+ consecutive reports (T04-02, T08-52, T09-40, T10-00, T21-40). Operator unblock confirmed by Mark Hallam on 2026-05-22. **Next claude-pool worker MUST claim this.**
**Sizing**: ~600 LOC across new crate (lib.rs + doc.rs + revision.rs + tests) + migration SQL + workspace `Cargo.toml` registration.

**Slot/v re-verification (2026-05-22 ledger snapshot — STILL run the pre-flight at claim time)**: latest slot is now **128** (`128__revoked_tokens.sql`), not 126 as the original brief noted. Latest v-registry is still **v76**. `EXPECTED_SCHEMA_VERSION` is still **76**. The slot number (next: ~129) is DIFFERENT from the v-version (next: v77) — they are not aligned on current main; verify via `scripts/check-migration-slots.sh` and inspect `registry_compliance.rs` for the slot↔v mapping convention before authoring.

## Why this brief exists

Read `docs/skycot/design/workplace-onboarding-architecture.md` §1 (operator's vision), §2 (Pillar 1 — Workplace Knowledge Repository), and §3.1 (data model — `workplace_knowledge_doc` + `workplace_knowledge_revision`).

The operator's framing: when a person joins a company they get the handbook, the playbook, training, and ongoing corrections. SKYCOT today gives an AI worker their persona file + vertical-hardcoded behaviors and nothing else. This is the substrate layer for the **handbook** — a tenant-scoped wiki of doc + revisions that other Wave-A briefs read from / write to / inject into agent system messages.

Without this substrate, briefs 2-6 cannot land. Brief 2 wraps audit kinds for the table; brief 3 puts a gateway in front; brief 4 puts a Next.js editor in front of the gateway; brief 5 reads from the table at every turn-start; brief 6 seeds the table on provision.

## Approach (concrete)

### Phase 1 — Migration

1. **Check the next free slot at claim time**:
   ```bash
   bash scripts/check-migration-slots.sh
   ls crates/argo-store/migrations/ | sort -V | tail -3
   ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -3
   ```
   At authoring time (origin/main `c613d922`), latest slot is **126** and latest v-registry is **v76**. **Verify these at claim time** — main may have moved (#1867 revocation has slot 127 fix-requested). Pick the next free slot (likely 127 or 128) and pair it with the next v-registry number.

2. **NEW** `crates/argo-store/migrations/<NN>__workplace_knowledge.sql` — paste the two tables from design doc §3.1 verbatim:

   ```sql
   -- workplace_knowledge_doc — one row per current head of a wiki doc.
   CREATE TABLE workplace_knowledge_doc (
       id                   TEXT      NOT NULL PRIMARY KEY,
       tenant_id            TEXT      NOT NULL,
       slug                 TEXT      NOT NULL,
       title                TEXT      NOT NULL,
       category             TEXT      NOT NULL,
       body_markdown        TEXT      NOT NULL,
       version              INTEGER   NOT NULL,
       parent_revision_id   TEXT,
       created_at_micros    INTEGER   NOT NULL,
       created_by           TEXT      NOT NULL,
       updated_at_micros    INTEGER   NOT NULL,
       updated_by           TEXT      NOT NULL,
       UNIQUE(tenant_id, slug)
   );

   CREATE INDEX idx_workplace_knowledge_doc_tenant_category
       ON workplace_knowledge_doc(tenant_id, category);

   CREATE TABLE workplace_knowledge_revision (
       id                TEXT      NOT NULL PRIMARY KEY,
       doc_id            TEXT      NOT NULL,
       tenant_id         TEXT      NOT NULL,
       version           INTEGER   NOT NULL,
       body_markdown     TEXT      NOT NULL,
       diff_from_prior   TEXT,
       edited_at_micros  INTEGER   NOT NULL,
       edited_by         TEXT      NOT NULL,
       edit_reason       TEXT
   );

   CREATE INDEX idx_workplace_knowledge_revision_doc
       ON workplace_knowledge_revision(doc_id, version DESC);
   ```

3. **NEW** `crates/argo-store/src/migrations/v<NN>.rs` — register the slot in the versioned runner. Mirror the pattern from `v76.rs` (single function, applies the .sql file, returns Result).

4. **MODIFY** `crates/argo-store/src/migrations/registry_compliance.rs` — add the entry `{ slot: <NN>, version: <NN>, file: "<NN>__workplace_knowledge.sql" }` to the array (find the existing entries for v74/v75/v76 as the template).

5. **MODIFY** `crates/argo-store/src/migrations/mod.rs` — register the v<NN> module (`pub mod v<NN>;` + add to the migrations vec).

6. **MODIFY** `crates/argo-store/src/lib.rs:341` — bump `EXPECTED_SCHEMA_VERSION` from 76 to <NN>.

### Phase 2 — New crate `argo-workplace-knowledge`

7. **NEW** `crates/argo-workplace-knowledge/Cargo.toml`:
   ```toml
   [package]
   name = "argo-workplace-knowledge"
   version = "0.0.1"
   edition = "2021"

   [dependencies]
   argo-store = { path = "../argo-store" }
   argo-types = { path = "../argo-types" }
   sqlx = { workspace = true }
   serde = { workspace = true, features = ["derive"] }
   serde_json = { workspace = true }
   thiserror = { workspace = true }
   uuid = { workspace = true, features = ["v7"] }

   [dev-dependencies]
   tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
   ```

8. **MODIFY** the workspace `Cargo.toml` (root) — add `"crates/argo-workplace-knowledge"` to the `members` list. Keep alphabetical-ish.

9. **NEW** `crates/argo-workplace-knowledge/src/lib.rs` (~150 LOC):
   - `pub mod doc;` (the head-doc CRUD module)
   - `pub mod revision;` (the revision-history module)
   - Re-export public types: `pub use doc::{WorkplaceKnowledgeDoc, Category};`
   - Crate-level doc comment with a `no_run` example mirroring the design doc's intended use.

10. **NEW** `crates/argo-workplace-knowledge/src/doc.rs` (~250 LOC):
    ```rust
    use argo_store::SessionStore;
    use argo_types::{ArgoResult, TenantId};
    use serde::{Deserialize, Serialize};
    use sqlx::SqlitePool;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub enum Category {
        Brand,
        BusinessRules,
        Compliance,
        Internal,
        Faq,
    }

    impl Category {
        pub fn as_db_str(&self) -> &'static str { /* "brand" | "business-rules" | ... */ }
        pub fn from_db_str(s: &str) -> Option<Self> { /* ... */ }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WorkplaceKnowledgeDoc {
        pub id: String,
        pub tenant_id: String,
        pub slug: String,
        pub title: String,
        pub category: Category,
        pub body_markdown: String,
        pub version: i64,
        pub parent_revision_id: Option<String>,
        pub created_at_micros: i64,
        pub created_by: String,
        pub updated_at_micros: i64,
        pub updated_by: String,
    }

    /// Create a new doc at version 1. Returns the new row.
    pub async fn create(
        pool: &SqlitePool,
        tenant_id: &TenantId,
        slug: &str,
        title: &str,
        category: Category,
        body_markdown: &str,
        created_by: &str,
    ) -> ArgoResult<WorkplaceKnowledgeDoc> { /* INSERT + return */ }

    /// Update head doc, increments version. Writes a revision row.
    /// Returns the new head.
    pub async fn update(
        pool: &SqlitePool,
        tenant_id: &TenantId,
        slug: &str,
        new_body: &str,
        updated_by: &str,
        edit_reason: Option<&str>,
    ) -> ArgoResult<WorkplaceKnowledgeDoc> { /* tx: write revision + update head + bump version */ }

    /// Soft delete: keeps revisions, removes head row.
    /// (Audit row is emitted by the caller, NOT this fn.)
    pub async fn delete(
        pool: &SqlitePool,
        tenant_id: &TenantId,
        slug: &str,
    ) -> ArgoResult<bool> { /* DELETE head; returns true if existed */ }

    /// Read head doc by slug. Returns None if not found (or wrong tenant).
    pub async fn get(
        pool: &SqlitePool,
        tenant_id: &TenantId,
        slug: &str,
    ) -> ArgoResult<Option<WorkplaceKnowledgeDoc>> { /* SELECT head */ }

    /// List all head docs for a tenant. Used by context-injection at turn-start.
    pub async fn list(
        pool: &SqlitePool,
        tenant_id: &TenantId,
    ) -> ArgoResult<Vec<WorkplaceKnowledgeDoc>> { /* SELECT all where tenant_id = ? */ }

    /// COUNT-only: returns max(version) across tenant for cache-key in
    /// the context-injection hot path. Returns 0 if tenant has no docs.
    pub async fn max_version_for_tenant(
        pool: &SqlitePool,
        tenant_id: &TenantId,
    ) -> ArgoResult<i64> { /* SELECT COALESCE(MAX(version), 0) WHERE tenant_id = ? */ }
    ```

11. **NEW** `crates/argo-workplace-knowledge/src/revision.rs` (~100 LOC):
    - `pub struct WorkplaceKnowledgeRevision { ... }` (mirrors row schema).
    - `pub async fn list_for_doc(pool, tenant_id, doc_id) -> ArgoResult<Vec<...>>`.
    - `pub async fn get(pool, tenant_id, revision_id) -> ArgoResult<Option<...>>`.
    - `pub async fn write(pool, tenant_id, doc_id, version, body, diff, edited_by, reason)` — internal to `doc::update`, made `pub(crate)` for now (gateway may need to read directly later).

### Phase 3 — Tests

12. **NEW** `crates/argo-workplace-knowledge/src/doc_tests.rs` (~150 LOC) included via `#[cfg(test)] mod tests;` in `doc.rs`. Coverage:
    - `create_and_get` — happy path.
    - `update_increments_version_and_writes_revision` — verifies the version counter + revision row count after 3 edits.
    - `delete_returns_true_only_if_existed` — first delete true, second delete false.
    - `list_filters_by_tenant` — insert docs under tenant A and tenant B; `list(A)` returns only A's docs (cross-tenant guard).
    - `get_with_wrong_tenant_returns_none` — slug exists in tenant A; `get(B, slug)` returns None.
    - `max_version_for_tenant_returns_zero_when_empty` — empty tenant edge case.

13. **NEW** `crates/argo-workplace-knowledge/src/revision_tests.rs` (~80 LOC):
    - `revisions_chained_by_parent_id` — after 3 edits, parent chain walks correctly.
    - `revision_list_ordered_desc_by_version` — newest first.

Every test uses `SessionStore::open(":memory:")` + `store.migrate()` (the same pattern as `argo-store/src/lib_tests.rs`).

## Files you may touch (verified to exist on `c613d922`)

- **NEW** `crates/argo-workplace-knowledge/Cargo.toml`.
- **NEW** `crates/argo-workplace-knowledge/src/lib.rs`.
- **NEW** `crates/argo-workplace-knowledge/src/doc.rs`.
- **NEW** `crates/argo-workplace-knowledge/src/doc_tests.rs`.
- **NEW** `crates/argo-workplace-knowledge/src/revision.rs`.
- **NEW** `crates/argo-workplace-knowledge/src/revision_tests.rs`.
- **NEW** `crates/argo-store/migrations/<NN>__workplace_knowledge.sql` (NN = next free slot at claim time).
- **NEW** `crates/argo-store/src/migrations/v<NN>.rs` (v<NN> = next free v at claim time).
- **MODIFY** `Cargo.toml` (workspace root) — add the new crate to `members`.
- **MODIFY** `crates/argo-store/src/migrations/mod.rs` — register the new v<NN> module.
- **MODIFY** `crates/argo-store/src/migrations/registry_compliance.rs` — add the slot entry.
- **MODIFY** `crates/argo-store/src/lib.rs:341` — bump `EXPECTED_SCHEMA_VERSION` to <NN>.

## Files you must NOT touch

- Any other crate's `src/`. This brief is substrate-only.
- `crates/argo-audit/src/event.rs` — AuditKind variants live in the sibling brief `mission-workplace-knowledge-audit-kinds`. Adding them here would race that brief.
- `crates/argo-cli/src/handlers/gateway/*` — endpoints live in brief 3 (`mission-workplace-knowledge-gateway`).
- `crates/argo-store/src/lib_tests.rs` — the schema_version assertion already uses `EXPECTED_SCHEMA_VERSION` post-#1918; bumping the constant in step 6 makes it automatically follow. Do NOT touch the test file.
- `crates/argo-store/src/migration_reconciliation_tests.rs` — same.
- `crates/argo-store/src/approvals_tests.rs` — same.
- Anything under `web/`.

## Pre-flight verification (run before authoring code)

```bash
# Confirm the next free migration slot — if it has moved, adjust step 1.
bash scripts/check-migration-slots.sh
ls crates/argo-store/migrations/ | sort -V | tail -5

# Confirm the next free v-registry number — should be one above the highest v*.rs.
ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -5

# Confirm cohort.rs exists in argo-store (sibling pattern — workplace_knowledge mirrors its tenant-scoping).
ls crates/argo-store/src/cohort.rs

# Confirm SessionStore::open(":memory:") pattern exists (test reference).
rg 'SessionStore::open\(":memory:"\)' crates/argo-store/src/lib_tests.rs | head -2

# Confirm EXPECTED_SCHEMA_VERSION line and value.
grep -n 'pub const EXPECTED_SCHEMA_VERSION' crates/argo-store/src/lib.rs

# Sanity — argo-types::TenantId is a thing.
rg 'pub struct TenantId' crates/argo-types/ | head -1
```

If any pre-flight returns unexpected (e.g. `EXPECTED_SCHEMA_VERSION` is no longer 76, or the migration slots have moved past 126), STOP and ping the conductor before authoring.

## Acceptance criteria

- [ ] New crate `argo-workplace-knowledge` compiles standalone:
  `cargo build -p argo-workplace-knowledge`.
- [ ] All test fns pass: `cargo test -p argo-workplace-knowledge` green (≥ 8 tests).
- [ ] Migration file at the next free slot; corresponding `v<NN>.rs`; `EXPECTED_SCHEMA_VERSION` bumped.
- [ ] `cargo test -p argo-store --lib` green (the schema_version assertions auto-track the bumped constant).
- [ ] `cargo clippy -p argo-workplace-knowledge -p argo-store --all-targets -- -D warnings` clean.
- [ ] `bash scripts/conductor-pre-merge-gate.sh` PASS (specifically gate-1 migration-slot check + gate-4 workspace clippy).
- [ ] Cross-tenant guard test passes: `list(A)` does NOT return docs from tenant B.
- [ ] No `unwrap` / `expect` outside `#[cfg(test)]` (CI invariant).
- [ ] Every modified file < 500 LOC (the largest expected: `doc.rs` at ~250 LOC).
- [ ] No new `pub struct` / `pub enum` / `pub type` with a name that already exists elsewhere in `crates/`. Verify with `rg 'pub (struct|enum|type) WorkplaceKnowledgeDoc' crates/` before committing.
- [ ] BUILD_STATUS.md row flipped to IN_REVIEW with PR number on push.

## Branch + PR

- Branch: `claude-mission-workplace-knowledge-substrate`
- PR title: `argo-workplace-knowledge: new crate + migration v<NN> for tenant-scoped wiki (mission-workplace-knowledge-substrate)`
- PR body must explicitly:
  - claim crate-scoped clippy + tests
  - claim the final `clippy --workspace` sanity check
  - declare any boundary deviations
  - tick every acceptance box

## Gotchas

- **Slot collision risk**: #1867 revocation has been fix-requested to use slot 127. If #1867 lands BEFORE this brief, take slot 128 (and v78). If this brief lands first, #1867 takes 128. Verify at claim time, do not hardcode.
- **`EXPECTED_SCHEMA_VERSION` bump**: this brief bumps it. The sibling test files (`lib_tests.rs`, `migration_reconciliation_tests.rs`, `approvals_tests.rs`) use the constant via `use crate::EXPECTED_SCHEMA_VERSION;` post-#1918, so they auto-track — do NOT edit them. If the assertions fail, you have a real bug, not a sweep target.
- **No `cohort` integration here**: Wave B will add `cohort.playbook_doc_id`. Do not pre-emptively add the column — that violates "wire or defer" (no consumer for it in Wave A).
- **`Category` enum**: keep it `serde`-compatible. The 5 values (brand / business-rules / compliance / internal / faq) are operator-locked per design doc §3.1; do not add more without a sibling brief.
- **uuid v7 for IDs**: monotonic + time-prefixed, the SKYCOT convention. Use `uuid::Uuid::now_v7()`.

## References

- Design doc: `docs/skycot/design/workplace-onboarding-architecture.md` §1, §2, §3.1, §4 (existing-substrate inventory), §7 (Pillar 1 acceptance).
- Sibling brief patterns:
  - `streams/codex-service-token-binding-stale-sweeper.md` — the "new pub fn on existing module + AuditKind variant" recipe.
  - `streams/argo-store-migration-assertion-refresh-v76.md` — the EXPECTED_SCHEMA_VERSION constant-import pattern.
- Existing tenant-scoped table for reference: `crates/argo-store/src/cohort.rs`.
- Versioned-doc pattern reference: `crates/argo-store/src/workflow_definition.rs` (design doc §4 calls this out as the parent-chain model).
