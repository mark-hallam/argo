# mvp-worker-trust-grade-column — add trust_grade + last_used_at to worker schema

**Owner**: claude. **Depends on**: nothing. **Tier M prereq — unblocks `mission-routing-skillmatch-rolematch` and `mission-skill-propagation-loop`.**

**Authored against**: origin/main 611c2854 on 2026-05-10.

## Why

Both Tier M briefs `mission-routing-skillmatch-rolematch` (M4) and
`mission-skill-propagation-loop` are BLOCKED because the codebase has
no notion of worker-side trust grade or recency. Today `TrustGrade`
(in `crates/argo-skill/src/trust.rs`) is a SKILL attribute derived
from skill provenance — there is no analogue on `Worker` /
`IdentityStore`. Without `worker.trust_grade` and `worker.last_used_at`
the routing engine cannot rank candidates for `TargetSelector::SkillMatch`
/ `RoleMatch` resolution.

This brief adds the two columns + accessor methods. **No routing-engine
integration in this brief** — that lands in the Tier M brief once this
prereq merges.

## Approach (concrete)

1. **Migration v48 + raw `48__worker_trust_grade.sql`** (priority slot;
   slot 47 is reserved for `mvp-identity-archive-worker` which was
   authored first). At claim time, run `ls crates/argo-store/migrations/`
   and `ls crates/argo-store/src/migrations/` — if v47 has already
   merged, slot 48 is correct; if v47 has NOT merged but is CLAIMED,
   still use 48 (siblings can land in either order). If 48 is also
   taken, use the next free slot and renumber both filenames + the
   versioned `mod.rs` match arm. Adds two columns:
   ```sql
   ALTER TABLE worker ADD COLUMN trust_grade INTEGER NOT NULL DEFAULT 0;
   ALTER TABLE worker ADD COLUMN last_used_at TEXT NULL;
   CREATE INDEX worker_trust_grade_idx ON worker (trust_grade DESC);
   CREATE INDEX worker_last_used_at_idx ON worker (last_used_at DESC);
   ```
   `trust_grade` semantics: `0=Unknown, 1=Builtin, 2=Tenant, 3=Reviewed,
   4=Published, 5=PublishedVerified` — mirrors `argo_skill::trust::TrustGrade`
   ordering so the same numeric scale carries across skill and worker.
2. **Versioned migration:** `crates/argo-store/src/migrations/v47.rs`
   following the v46 pattern (read `v46.rs` for shape).
3. **Schema-tip assertions** in `lib_tests.rs` and
   `migration_reconciliation_tests.rs` bumped 46 → 47 (EXHAUSTIVE-MATCH).
4. **`IDENTITY_SCHEMA` in `argo-identity/src/store_sql.rs`** — append
   the two columns to the inline CREATE TABLE so fresh-DB path matches
   versioned-DB path.
5. **Identity API:**
   - `IdentityStore::set_worker_trust_grade(worker_id, grade) -> Result<()>`
   - `IdentityStore::touch_worker_last_used(worker_id) -> Result<()>`
     (sets `last_used_at = now()`)
   - Extend `Worker` struct in `argo-identity` with `trust_grade: u8`
     + `last_used_at: Option<DateTime<Utc>>`. Populate in
     `worker_from_row` helper.
6. **5 unit tests** in `argo-identity`:
   - `set_worker_trust_grade_persists`
   - `touch_worker_last_used_sets_timestamp`
   - `worker_default_trust_grade_is_zero`
   - `list_workers_returns_trust_grade_field`
   - `migration_v47_applies_to_v46_db`

## Files you may touch

- `crates/argo-store/migrations/48__worker_trust_grade.sql` (NEW; rename if 48 taken at claim time)
- `crates/argo-store/src/migrations/v48.rs` (NEW, ≤ 80 LOC; rename if 48 taken)
- `crates/argo-store/src/migrations/mod.rs` (1 mod decl, 1 match arm)
- `crates/argo-store/src/lib_tests.rs` (schema-tip 46→48 if 47 already merged, else 46→48 directly)
- `crates/argo-store/src/migration_reconciliation_tests.rs` (schema-tip)
- `crates/argo-identity/src/store_sql.rs` (extend IDENTITY_SCHEMA)
- `crates/argo-identity/src/store.rs` (new methods + worker_from_row)
- `crates/argo-identity/src/lib.rs` (re-exports if needed)
- Any `Worker { ... }` struct-init callsite workspace-wide ONLY to add
  the new fields under EXHAUSTIVE-MATCH amendment

## Files you must NOT touch

- `crates/argo-orchestrator/` — routing-engine integration is the
  follow-up brief, not this prereq
- `crates/argo-skill/src/trust.rs` — keep as skill-side TrustGrade
- Any vertical manifest

## Acceptance criteria

- [ ] Migration v48 (or whichever slot was free at claim time) applies cleanly on fresh DB AND on the prior-version-state DB
- [ ] `set_worker_trust_grade` + `touch_worker_last_used` work
- [ ] `Worker` struct exposes `trust_grade: u8` + `last_used_at: Option<...>`
- [ ] 5 unit tests pass
- [ ] `cargo clippy -p argo-store -p argo-identity --all-targets -- -D warnings` clean
- [ ] No file crosses 500 LOC

## Branch + PR

- Branch: `claude-mvp-worker-trust-grade-column`
- PR title: `identity: trust_grade + last_used_at columns on worker (mvp-worker-trust-grade-column)`
