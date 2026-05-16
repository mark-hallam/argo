# codex-migration-monotonic-renumber — Renumber migrations to strictly monotonic + add pre-sqlx validator (F-02 P0)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `ad581eca` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-migration-monotonic-renumber`.
**Source**: F-02 P0 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`. Best-practice operator decision (Q4 2026-05-15): rename to canonical (industry norm).

## Scope

🔴 P0 — Schema apply order is structurally ambiguous. 11 slot collisions in `crates/argo-store/migrations/`:

| Slot | Files |
|---|---|
| 19 | `19__anomaly.sql`, `19__worker_fs_root.sql` |
| 22 | `22__tenant_archived.sql`, `22__roster.sql` |
| 23 | `23__handoff_routing.sql`, `23__workflow_checkpoints.sql` |
| 24 | `24__skill_quality_score.sql`, `24__operator_feedback.sql`, `24__vertical_archived.sql` (TRIPLE) |
| 26 | `26__worker_budget_alert.sql`, `26__spend_baseline.sql`, `26__data_classification.sql` (TRIPLE) |
| 27 | `27__handoff_auctions.sql`, `27__spend_anomaly.sql` |
| 28 | `28__tenant_quota.sql`, `28__recovery_cache.sql`, `28__role_duties.sql` (TRIPLE) |
| 30 | `30__learned_plan_templates.sql`, `30__worker_charter.sql` |
| 45 | `45__cu_screenshot_index.sql`, `45__worker_dependency.sql`, `45__worker_level.sql` (TRIPLE) |
| 51 | `51__audit_worker_deleted_at.sql`, `51__worker_skill.sql` |
| 75 | `75__scim_provisioned_tombstones.sql`, `75__siem_dispatch_cursor.sql` |

The migration-chain test has special duplicate-column suppressions to hide this drift (`crates/argo-store/tests/migration_chain.rs:35`, `:320`, `:325`).

Per operator Q4 (2026-05-15): rename to canonical monotonic order. Industry standard for SQL migration tools (Diesel, sqlx, Alembic, Rails).

Scope:

- Renumber the 19 files in collision groups to monotonically-increasing slots starting from the next available number after the highest existing slot (the highest non-colliding slot becomes the floor; new slots go above).
- Update the Rust slot_validator registry to reflect new numbers.
- Update any code/tests that reference old slot numbers literally (grep for `19__anomaly`, etc.).
- Add `crates/argo-store/src/migrations/duplicate_slot_validator.rs` (NEW) — a pre-sqlx validator that walks the migrations directory, parses filename slot prefixes, and panics-or-fails-build if any duplicate exists. Run this in `migrate()` before sqlx applies anything.
- Delete the duplicate-column suppressions in `migration_chain.rs:35`, `:41`, `:42`, `:320`, `:325` AFTER confirming the reconciliation test still passes.
- Add a migration-chain test that proves: (a) no duplicate slots, (b) monotonic ordering, (c) validator fails-fast on simulated duplicate.

LOC: ~250 (19 file renames + slot registry edits + validator + tests).

## Files you may touch

- `crates/argo-store/migrations/*.sql` — RENAME the 19 colliding files via `git mv` to new monotonic slot numbers. SQL content unchanged. ≤100 modified lines (mostly renames, no content edits).
- `crates/argo-store/src/migrations/slot_validator.rs` — update slot registry to match new numbers. ≤80 modified lines.
- `crates/argo-store/src/migrations/duplicate_slot_validator.rs` (NEW) — pre-sqlx duplicate-slot validator. ≤60 LOC.
- `crates/argo-store/src/migrations/mod.rs` — wire the new validator. ≤15 modified lines.
- `crates/argo-store/tests/migration_chain.rs` — remove duplicate-column suppressions; add monotonic-order test. ≤60 modified lines.
- `crates/argo-store/migrations_test.rs` (if exists) or sibling — validator regression test. ≤40 LOC.
- Any Rust source files that reference old slot numbers literally (grep with `rg '19__|22__|23__|24__|26__|27__|28__|30__|45__|51__|75__' crates --type rust`). ≤50 modified lines total.

## Files you must NOT touch

- SQL CONTENT of the migrations. Rename only — don't modify the SQL itself; that would change schema semantics.
- Other store/identity/orchestrator crate internals beyond slot-number references.
- Production `seed-*.sql` files that customers may have applied — those need rollback safety beyond this brief's scope.

## Acceptance criteria

- [ ] `ls crates/argo-store/migrations/ | awk -F'__' '{print $1}' | sort | uniq -d` returns NOTHING.
- [ ] `slot_validator.rs` registry matches filesystem after rename.
- [ ] `duplicate_slot_validator.rs` exists; `migrate()` invokes it before sqlx; validator panics on simulated duplicate slot.
- [ ] Duplicate-column suppressions in `migration_chain.rs` are deleted.
- [ ] Monotonic-order regression test passes.
- [ ] `cargo test -p argo-store` passes.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] PR body lists the OLD → NEW slot mapping for every renamed file (operator audit trail).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-migration-monotonic-renumber`
- PR title: `fix(store): renumber migrations to monotonic + add pre-sqlx validator (codex-migration-monotonic-renumber) — F-02 P0`

## Notes for the implementer

- **DO NOT change SQL content during rename.** Renumbering only changes apply order, not schema. Changing both at once makes recovery impossible if something breaks.
- The current `migration_chain` test was specifically designed to tolerate collisions. After this brief, it must enforce monotonicity. The test's `tests.rs:35`/`:41`/`:42`/`:320`/`:325` suppressions all need deletion.
- For deployed customers (if any): a separate migration-recovery brief may be needed to map old→new slot identity in their argo_meta tables. Flag in PR body if reviewer needs to consider.
- The new validator should be called BEFORE sqlx::migrate runs, so the panic happens cleanly at startup, not mid-application.
- Per operator Q4 framing ("best practice regardless of time"), this is the correct architectural fix even though it's higher-touch than a comment + registry-only workaround.
