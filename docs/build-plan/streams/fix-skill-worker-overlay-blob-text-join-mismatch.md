# fix-skill-worker-overlay-blob-text-join-mismatch

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `963981405` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skill-worker-overlay-blob-text-join-mismatch`.

🔴 **P1 CLAIM-4 PRODUCTION DATA-INTEGRITY BUG (4 evaluator rounds RED).** Per-worker skill overlays the cron promotes are silently invisible to the worker's next session — direct mission-claim-4 hit. The bug was identified by evaluator round 41 and re-confirmed RED on origin/main by rounds 42 / 43 / 44.

## Why

The bug is a column-type mismatch between two SQL migrations:

- `crates/argo-store/src/migrations/v1.rs:107` — `skill.id` is **BLOB** (16 raw bytes).
- `crates/argo-store/src/migrations/101__worker_skill.sql:28` — `worker_skill.skill_id` is **TEXT** (UUID string form).

The read path joins them at `crates/argo-skill/src/store_lookups.rs:118-119`:

```sql
SELECT ... FROM skill s JOIN worker_skill ws ON s.id = ws.skill_id ...
```

SQLite's type-affinity does NOT coerce across BLOB↔TEXT for `=` — the join returns **0 rows** for every per-worker overlay. The write path at `crates/argo-skill/src/per_worker_learning_cron.rs:237` binds `.bind(skill_id.to_string())` (TEXT form) into `worker_skill.skill_id` — so the row is persisted, just unreachable.

Symptom on origin/main: `cargo test -p argo-skill --lib store_worker_overlay_tests::` returns **3 pass + 2 FAIL** on every recent HEAD (4 consecutive evaluator rounds: 41 / 42 / 43 / 44). The two failing tests correctly assert the intended behaviour — they are not flakes.

## Scope

**Two viable paths — pick A unless the migration-window cost is small.**

### Path A — read-path coercion (smaller, ~80 LOC)

1. Modify the read-side JOIN in `store_lookups.rs` to coerce the column types: `... ON hex(s.id) = lower(ws.skill_id) ...` OR `... ON s.id = unhex(ws.skill_id) ...` (whichever SQLite syntax matches v3.45).
2. Add a regression test that proposes a skill via the same write path the cron uses, then asserts `list_per_worker_overlay(...)` finds it.
3. Un-`#[ignore]` (or wire-up the un-flagged) `store_worker_overlay_tests` so the 2/5 RED tests are now expected to pass.

### Path B — schema migration to TEXT (~200 LOC + migration slot)

1. Add migration `71__worker_skill_skill_id_to_blob.sql` (or 72+ per the slot pin in BUILD_STATUS.md) — `ALTER TABLE worker_skill RENAME TO worker_skill_old; CREATE TABLE worker_skill (... skill_id BLOB ...); INSERT INTO worker_skill SELECT id, worker_id, unhex(replace(skill_id, '-', '')), ...`.
2. Update write path at `per_worker_learning_cron.rs:237` to bind raw bytes.
3. Same regression test + un-ignore.
4. Pre-existing rows in TEXT form must round-trip cleanly — include a migration smoke test loading a fixture.

## May touch

Path A:
- `crates/argo-skill/src/store_lookups.rs`
- `crates/argo-skill/src/store_worker_overlay_tests.rs` (or wherever the existing failing tests live)
- One new regression test file under `crates/argo-skill/tests/`

Path B (additionally):
- `crates/argo-store/src/migrations/7N__worker_skill_skill_id_to_blob.sql` (slot per BUILD_STATUS.md pin)
- `crates/argo-store/src/migrations/registry.rs` (slot registration)
- `crates/argo-skill/src/per_worker_learning_cron.rs`

## Acceptance

- [ ] `cargo test -p argo-skill --lib store_worker_overlay_tests` 5/5 green on origin/main.
- [ ] New regression test in `crates/argo-skill/tests/` that exercises propose → cron-promote → worker-session-load path end-to-end and asserts overlay-visibility.
- [ ] `cargo clippy -p argo-skill -p argo-store --all-targets -- -D warnings` clean.
- [ ] Path B only: migration-slot pin in `BUILD_STATUS.md` updated; `scripts/check-migration-slots.sh` exit 0.
- [ ] PR body explicitly states which path (A or B) was chosen and why.

## Why not just delete the failing tests

The 2/5 RED tests pin the production contract; deleting them would silence the bug. The fix is to make the production path do what the tests already assert it should.

## Why this stayed silent on main

`scripts/conductor-pre-merge-gate.sh` gates 1-5 run clippy + LOC budget + zero-caller-substrate + migration-slot collision + workspace-clippy, but **none** run `cargo test`. The `--skip-clippy` admin-merge pattern shorts gate 5 entirely. Round-44 P0'd `ops-pre-merge-gate-fix-test-failures-not-just-clippy` to add gate 6 (cargo-test); when that lands together with this fix, no future row of this kind can carry silently across 6+ PRs.

## Background

- Bug discovered by evaluator round 41 (`docs/skycot/evals/scheduled/2026-05-23/round-41-worker-skill-overlay-blob-text-join-mismatch-verify.md`).
- Re-confirmed RED rounds 42 / 43 / 44.
- Original ledger row for this brief landed `tick-121` (`2026-05-23T~15Z`) but the streams/ file was never committed — this re-author closes that gap.
