# Mission load-bearing wire — skill.min_trust_grade column (M6 precursor 1/3)

**Owner**: claude. **Depends on**: `skill` table SHIPPED; `argo-skill::trust::TrustGrade` enum SHIPPED.
**Authored against**: origin/main `5e1ef545` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-min-trust-grade-column`.

> SOURCE: precursor authored 2026-05-11 by conductor to unblock `mission-skill-propagation-loop` (Tier M6 — v1 critical path). The M6 brief was BLOCKED because it assumed a `min_trust_grade` field on skill rows that doesn't exist. This brief adds the column + a setter API; the next brief (mission-skill-store-available-for-worker) adds the lookup; the third (mission-skill-session-boot-wire) wires it into worker session boot. Then the rewritten M6 builds on all three.

## Scope

Migration that adds `min_trust_grade INTEGER NULL` to the `skill` table. NULL means "no minimum" (any worker can use). When set, only workers whose own trust_grade is ≥ this value can discover the skill (enforcement in the M6 chain).

Also add typed setter/getter on `SkillStore`:

```rust
pub async fn set_min_trust_grade(&self, name: &str, grade: Option<TrustGrade>) -> ArgoResult<()>;
pub async fn get_min_trust_grade(&self, name: &str) -> ArgoResult<Option<TrustGrade>>;
```

The integer column stores the `TrustGrade` numeric value (lower = less trusted). NULL = no restriction. The Rust API converts between `TrustGrade` enum and the underlying integer.

CLI: extend `argo skill update <name> --min-trust-grade <grade>` (existing skill update handler at `crates/argo-cli/src/handlers/skill_update.rs` or similar — verify path in first 5min).

This is NOT in scope: enforcement logic (M6 chain); discovery filtering (M6 chain); cross-tenant trust composition (separate brief mission-cross-tenant-trust-graph).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__skill_min_trust_grade.sql` (NEW) — `ALTER TABLE skill ADD COLUMN min_trust_grade INTEGER NULL;`. Idempotent via `column_exists` PRAGMA probe. **Implementer MUST verify next free slot** via `grep -E "v4[6-9]|v5[0-9]" docs/build-plan/BUILD_STATUS.md` to avoid collision with in-flight briefs.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned migration.
- `crates/argo-skill/src/store.rs` — narrowly scoped: ADD `set_min_trust_grade` + `get_min_trust_grade` methods near existing `set_lock`. Reuse existing PRAGMA/SQL patterns. Verify file currently <450 LOC; if at-risk-of-ceiling, place new methods in a NEW sibling `store_trust.rs`.
- `crates/argo-skill/src/store_trust_tests.rs` (NEW, sibling) — 5 unit tests: set-roundtrip; null-default; set-to-null-clears; invalid-name-errors; invalid-grade-rejected.
- `crates/argo-cli/src/handlers/skill_update.rs` (or wherever `argo skill update` lives — verify in first 5min) — narrowly scoped: ADD `--min-trust-grade <grade>` flag pass-through.

## Files you must NOT touch

- `crates/argo-skill/src/trust.rs` — TrustGrade enum is upstream; reuse only
- `crates/argo-skill/src/quality.rs` — quality is orthogonal
- Any non-store/non-skill/non-cli crate
- Existing migration files

## Acceptance criteria

- [ ] Migration adds `min_trust_grade INTEGER NULL` column idempotently
- [ ] Schema-tip bump in lib_tests / migration_reconciliation_tests / approvals_tests
- [ ] `SkillStore::set_min_trust_grade(name, Some(Grade))` writes; `Some` round-trips
- [ ] `set_min_trust_grade(name, None)` clears to NULL
- [ ] `get_min_trust_grade` returns `None` for skills with NULL column
- [ ] `argo skill update <name> --min-trust-grade <grade>` CLI works
- [ ] 5 unit tests
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-skill -p argo-cli` clean
- [ ] `cargo clippy -p argo-store -p argo-skill -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-skill -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-min-trust-grade-column`
- PR title: `skill: min_trust_grade column + setter/getter (mission-skill-min-trust-grade-column)`

## Notes for the implementer

- The column is INTEGER NULL on purpose — NULL = "no restriction" is the v1 default, and operators opt in to gating by setting a value.
- The TrustGrade enum has a documented numeric ordering (lower index = less trusted). Use the existing `TrustGrade::as_u8()` / `from_u8()` if present, or implement via discriminant if not. Don't invent a new mapping.
- This is the **first** in a 3-brief chain: precursor 1/3 (this) → 2/3 (`mission-skill-store-available-for-worker` — a lookup method) → 3/3 (`mission-skill-session-boot-wire` — wires the filter into worker boot). Then the rewritten `mission-skill-propagation-loop` builds the demo on all three.
- Migration slot: check #1271 (v45), #1273 (v46), and any in-flight before claiming a number. Likely v47 or v48 is free.
