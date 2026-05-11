# Mission load-bearing wire — SQLite WAL tuning for workload pattern

**Owner**: claude. **Depends on**: existing SQLite store opener (verify in first 5min: `grep -rn 'SqlitePool\|sqlx::sqlite\|PRAGMA' crates/argo-store/src/`); workspace runs on SQLite (per CLAUDE.md "single-binary architecture").
**Authored against**: origin/main `b908e563` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sqlite-wal-tuning`.

> SOURCE: MISSION_BACKLOG.md Section 7 #239 — "SQLite WAL tuning per workload pattern." Argo's SQLite store uses default pragmas. Under multi-worker load the audit-record + session insert pattern is write-heavy + bursty; WAL with default checkpoint thresholds creates large `*.db-wal` files (100MB+) and stutters during auto-checkpoint. Mission promise: opinionated WAL pragmas tuned for the observed workload (high-write-burst + occasional-long-read).

## Scope

1. **NEW pragma set** applied at `SessionStore::open` (or wherever the pool is constructed). Add these in order:
   ```
   PRAGMA journal_mode = WAL
   PRAGMA synchronous = NORMAL          -- default is FULL; NORMAL is safe with WAL
   PRAGMA wal_autocheckpoint = 1000     -- pages, ~4MB at 4k pagesize; default is 1000 but make explicit
   PRAGMA cache_size = -64000           -- 64 MB negative=KiB
   PRAGMA mmap_size = 268435456         -- 256 MB mmap window
   PRAGMA temp_store = MEMORY
   PRAGMA busy_timeout = 5000           -- ms; multi-worker contention
   ```

2. **Per-pragma rationale doc** in `crates/argo-store/src/wal_tuning.rs` rustdoc — operator-readable summary of the trade-off each pragma makes.

3. **`SessionStore::open` extension**: accept optional `WalTuningProfile { Conservative, Default, Aggressive }`:
   - `Conservative`: default SQLite pragmas (current behavior).
   - `Default` (v1 default): the 7 pragmas above.
   - `Aggressive`: same as Default + `synchronous = OFF` + `wal_autocheckpoint = 10000`. **Documented as dev-only** with a `tracing::warn!` on open.

4. **Config knob**: `[store.wal_tuning]` in `argo.toml`:
   ```toml
   [store.wal_tuning]
   profile = "default"   # "conservative" | "default" | "aggressive"
   ```
   Missing block → "default" (preserve forward-looking improvement).

5. **NEW CLI**: `argo doctor wal-stats [--json]` — runs `PRAGMA wal_checkpoint(PASSIVE)` + reads `*.db-wal` size + reports `journal_mode` / `synchronous` / `cache_size` / `mmap_size`. Operator inspection.

6. **Test**: assert all 7 pragmas applied via `PRAGMA <name>` round-trip. Mock-aware so the same test runs on `:memory:`.

This is NOT in scope: cross-platform WAL behavior tuning (Linux/macOS/Windows treat fsync differently — accept defaults); incremental_vacuum or other space-reclaim strategies; switching off WAL entirely (still useful for many readers).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/wal_tuning.rs` (NEW, ≤ 250 LOC) — `WalTuningProfile` enum + `apply_pragmas` fn + docstring.
- `crates/argo-store/src/wal_tuning_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests: default-profile-applies-all; conservative-applies-none; aggressive-warns; pragma-roundtrip-via-memory-db; missing-config-defaults; bad-profile-string-errors.
- `crates/argo-store/src/lib.rs` — narrowly scoped: extend `SessionStore::open` signature OR add `open_with_profile` variant. ≤ 25 LOC delta.
- `crates/argo-config/src/store.rs` (verify path) — narrowly scoped: ADD `[store.wal_tuning]` parser. ≤ 25 LOC delta.
- `crates/argo-cli/src/handlers/doctor.rs` (verify path) — narrowly scoped: ADD `WalStats` subcommand handler or wal-stats section. ≤ 50 LOC delta.

## Files you must NOT touch

- Any other `crates/argo-store/src/` file beyond what's listed
- `crates/argo-store/migrations/**` — no schema change
- Any non-store/non-config/non-cli crate

## Acceptance criteria

- [ ] All 7 pragmas applied at `SessionStore::open` with Default profile
- [ ] Conservative profile leaves SQLite defaults intact
- [ ] Aggressive profile emits `tracing::warn!` on open
- [ ] `argo.toml [store.wal_tuning].profile` parses; missing → Default
- [ ] `argo doctor wal-stats` reports pragma state + WAL file size
- [ ] Existing argo-store tests pass byte-identical (no behavioral regression in :memory: mode)
- [ ] 6 unit tests
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-config -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-sqlite-wal-tuning`
- PR title: `store: opinionated WAL pragmas + Conservative/Default/Aggressive profiles (mission-sqlite-wal-tuning)`

## Notes for the implementer

- The pragma values above are **starting points**. Document them as such — future tuning may adjust based on production telemetry.
- The `synchronous = NORMAL` change is safe with WAL but operators on flaky hardware may prefer Conservative. Document this trade-off prominently.
- The `mmap_size = 256MB` will be silently capped on 32-bit systems; SQLite handles this gracefully. Document in PR body.
- For `:memory:` DBs: WAL pragma is irrelevant — wal_tuning shouldn't crash on memory pools. Test this explicitly.
- The Aggressive profile is **dev-only**. The warn line is load-bearing — don't suppress.
