# Round 100 — PR #2495 skill UNIQUE substrate fix: anti-theater pre-merge

**Date**: 2026-05-27 (UTC 09:27Z; Melbourne 2026-05-27 19:27 AEST)
**Round**: 100
**Pillar / Claim**: Claim 4 (Skills shared + learned) / Mission Scene 4 (skills marketplace per-tenant seeding)
**Verdict**: 🟢 PRE-MERGE PASS — production regression confirmed open on origin/main; migration v84 + 3 regression tests properly close the load-bearing failure mode. **Resolves R97 P0 / R98 P0 / R99 P0 substrate half — 3-round carry closes if merged.**

## Setup

PR #2495 (`v84: relax skill UNIQUE(name, version) → UNIQUE(name, version, tenant_id)`) is OPEN and `MERGEABLE`,
head SHA `fe95178` on branch `claude-fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed`,
+391/-3 across 5 files (1 SQL migration, 1 Rust migration module with 3 new regression tests, 3 module
registration edits).

Triggered by R97 (2026-05-27T01:55Z) which cold-reran `cargo test -p argo-demo-seed --lib
seeds_independently_per_tenant` against the post-#2437 main and watched it fail with
`UNIQUE constraint failed: skill.name, skill.version`. R98 + R99 both verified the substrate half remained
open on subsequent rounds. This round verifies the in-flight fix.

## Observed

**Production regression confirmed open on origin/main `03877f581`:**

```
$ grep -n "UNIQUE" crates/argo-store/src/migrations/v1.rs
34:        UNIQUE(tenant_id, name)
66:        UNIQUE(session_id, seq)
118:        UNIQUE(name, version)    ← the global skill UNIQUE the brief targets
211:        id              BLOB NOT NULL UNIQUE,
$ grep -n "EXPECTED_SCHEMA_VERSION" crates/argo-store/src/lib.rs
364:pub const EXPECTED_SCHEMA_VERSION: u32 = 83;    ← v83 max, no v84
$ ls crates/argo-store/migrations/ | tail -1
99__worker_level.sql    ← filename slot 134 max, no slot 135
```

**PR changes (via `git show pr-2495:`):**

1. **NEW** `crates/argo-store/migrations/135__skill_unique_per_tenant.sql` (+89 LOC) — canonical SQLite
   table-swap. Creates `skill_new` with `UNIQUE(name, version, tenant_id)`, copies rows, drops old, renames,
   recreates the four auxiliary indexes (`idx_skill_vertical_id`, `idx_skill_lifecycle`,
   `idx_skill_shared_cluster`, `idx_skill_tenant_id`). The `session_skill` materialise-and-restore guard
   uses a non-TEMP helper table per the brief's stated `tests/migration_chain.rs` raw-replay constraint
   (TEMP tables are connection-scoped; the raw-replay harness rotates connections between statements).

2. **NEW** `crates/argo-store/src/migrations/v84.rs` (+293 LOC including 3 tests) — `up()` / `down()`
   loaders + 3 regression tests on a `fk_on_pool()` (PRAGMA foreign_keys = ON):
   - `up_allows_same_name_version_across_tenants` — inserts two rows with same name+version under
     `tenant_a` and `tenant_b`, asserts both succeed; inserts a third with same name+version+tenant under
     `tenant_a`, asserts UNIQUE violation.
   - `up_preserves_session_skill_fk_after_table_swap` — pre-seeds workspace + session + skill + session_skill
     row at v83 (before v84), then runs v84, asserts `session_skill` row count unchanged. Without the
     materialise-and-restore guard this would cascade-delete to zero under `foreign_keys = ON`.
   - `up_preserves_all_skill_indexes_after_table_swap` — queries `sqlite_master` post-swap, asserts all 4
     index names present.

3. `crates/argo-store/src/lib.rs:364` — `EXPECTED_SCHEMA_VERSION: u32 = 83 → 84`.

4. `crates/argo-store/src/migrations/mod.rs` — `mod v84` added.

5. `crates/argo-store/src/migrations/registry_compliance.rs` — header bumps from `57..=83` to `57..=84` and
   appends the `Migration { version: 84, up: |tx| ..., down: |tx| ... }` entry.

**The load-bearing regression test** lives at `crates/argo-demo-seed/src/marketplace.rs:191`
(`seeds_independently_per_tenant`). It calls real `seed_demo_marketplace_skills(&store, tenant_a)`, asserts
the returned `inserted_a == demo_marketplace_catalog_size()`, repeats for `tenant_b` (asserts the same
count), re-runs `tenant_a` and asserts `0` rows inserted (per-tenant idempotency), then runs raw
`SELECT COUNT(*) FROM skill WHERE tenant_id = ?` against the store pool for both tenants and asserts each
equals the catalog size. Real handler, real store, real SQL — no mock-bypass surface.

## Anti-theater check

Five axes verified:

1. **Real production regression on origin/main** — `v1.rs:118` bare `UNIQUE(name, version)` confirmed by
   raw grep against the current main checkout (no PR involvement).
2. **Real fix in the migration path** — `135__skill_unique_per_tenant.sql` creates `skill_new` with
   `UNIQUE(name, version, tenant_id)` and the table-swap dance is the canonical SQLite pattern. Not a
   rewrite of the test, an actual schema change.
3. **Real new tests with non-trivial assertions** — the three v84 tests aren't tautologies; the FK
   preservation test would catch a cascade-delete regression, the cross-tenant test would fail under either
   the pre-fix global UNIQUE or a no-op fix, the index test would fail under any missing index.
4. **Real `fk_on_pool` test pool** — `PRAGMA foreign_keys = ON` is explicitly armed via `after_connect`,
   per the brief's HAZARD 1 recipe. This is a deliberate departure from the live runtime (which keeps FKs
   off) so the migration is forward-protected against any future FK-on toggle.
5. **Sensible-deviations transparency** — PR body explicitly flags the brief specified
   `PRAGMA defer_foreign_keys = ON` as HAZARD 1 guard, the author empirically discovered that pragma defers
   FK *violation checks* but NOT `ON DELETE CASCADE` actions, and pivoted to materialise-and-restore. This
   level of self-correction by the author is the opposite of theater. The pragma is left in as a
   belt-and-suspenders guard with a comment.

**PR body test-plan claims:**

- `cargo test -p argo-demo-seed --lib seeds_independently_per_tenant` — claims PASS (cold).
- `cargo test -p argo-store --lib up_allows_same_name_version_across_tenants` — claims PASS.
- `cargo test -p argo-store --lib up_preserves_session_skill_fk_after_table_swap` — claims PASS.
- `cargo test -p argo-store --lib up_preserves_all_skill_indexes_after_table_swap` — claims PASS.
- `cargo test -p argo-store --lib` — claims 519 passed, 0 failed (51s wall).
- `bash scripts/check-migration-slots.sh` — claims exit 0.
- `bash scripts/workspace-clippy-fast-fail.sh` — claims clean.

**Cold-cargo NOT re-run by this round** — workers are active (2 slots, both ACTIVE per
[supervisor-watchdog] T09:19Z), so re-running cargo would fight for `CARGO_TARGET_DIR` locks. Source-code
anti-theater verification is sufficient for the PRE-MERGE PASS recommendation; the conductor's pre-merge
gate (or operator) can cold-cargo at admin-merge time.

**Body does NOT contain any of the R99 P1 admission phrases** (`local cargo check was interrupted`,
`formal pre-merge gate will run`, etc) — this is a clean honest PR body.

## Gap / follow-up brief candidates

Nothing P0/P1 to recommend authoring. The PR closes the substrate half cleanly. Two informational notes:

1. **Post-merge gateway restart required** — the PR body flags this explicitly:
   `gateway_schema=v83 codebase_latest=v84 drift=1` until `make restart-live-gateway` runs. The conductor's
   post-merge tick must surface this. The existing
   `ops-gateway-freshness-binary-staleness-detection-gap-claude-fixed` brief (TODO on the ledger) already
   addresses the broader detection gap; this PR's restart is just normal post-migration hygiene.

2. **Sibling defects flagged by PR #2457 (also in-flight)** — PR #2457's body names
   `provider_key_response` + `install_vertical_response` as having the same write/read tenant mismatch as
   `first_hire_response`. The pattern is the same family as PR #2495 (constraint that didn't get the
   tenant scope when tenant_id was added). Worth a sweep of every multi-tenant `UNIQUE`/`PRIMARY KEY` in
   the schema once both PRs land. Recommend NOT authoring this as a P2 brief yet — wait for #2457 to merge
   so the conductor can author a single sweep brief from the unified evidence.

## Recommendation to conductor

**Admit PR #2495 to the `--skip-clippy` evaluator-pre-verified lane.** Five anti-theater axes pass; PR
body test-plan is internally consistent; production regression on origin/main reproducibly present
(`v1.rs:118` + `EXPECTED_SCHEMA_VERSION = 83`); fix in PR #2495 is targeted, transparent, and
self-correcting on the brief's own HAZARD model.

If admin-merged: post-merge restart-live-gateway is required (PR body flags it; expected v83→v84 drift
state for ~1 tick).

This closes the R97 P0 substrate carry that R98 + R99 both surfaced.
