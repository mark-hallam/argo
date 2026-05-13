# claude-store-migrations-mod-split — extract migration registry from mod.rs

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `d57d3a31` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-store-migrations-mod-split`.

## Scope

Pure refactor. `crates/argo-store/src/migrations/mod.rs` is at **493 LOC** (7 LOC of headroom against the 500-ceiling). This blocks **6 in-queue mission briefs** that need to register a new migration here:

- mission-gateway-request-audit-log
- mission-audit-chain-watchdog
- mission-planner-bias-from-template
- mission-skill-explanation-provenance
- mission-operator-feedback-dreams-curation
- mission-multi-vertical-concurrency-lock

The file's bulk is the `all_migrations() -> Vec<Migration>` function (line 162) which holds a giant ordered `Vec` literal of every migration version. The 50+ `mod vN;` declarations at the top can stay — they're 1 line each and idiomatic.

Split target: extract `all_migrations()` (and any private helpers it calls) into a sibling `crates/argo-store/src/migrations/registry.rs`. Drops `mod.rs` from 493 → ~50 LOC.

## Approach

1. Create `crates/argo-store/src/migrations/registry.rs` containing the `all_migrations()` function verbatim plus whatever `use` lines + `Migration` type imports it needs.
2. In `mod.rs`, replace the function with `pub(crate) use registry::all_migrations;`. Keep the `mod vN;` declarations + the `ensure_schema_table` helper at line 136 (load-bearing for boot).
3. Add `mod registry;` line in `mod.rs`.

After split:
- `mod.rs` ≤ 100 LOC (target: ~50) — just `mod vN;` declarations + `ensure_schema_table` + the `pub(crate) use` re-export
- `registry.rs` ≤ 450 LOC

Headroom: each new migration is a single `Migration { version: N, ... }` literal in `registry.rs`. The registry file will eventually hit the ceiling too, but it gives us ~50 new migrations of runway.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/migrations/mod.rs` — extract `all_migrations()` body to sibling; replace with re-export.
- `<NEW: crates/argo-store/src/migrations/registry.rs>` — receives `all_migrations()` verbatim.
- No other files.

## Files you must NOT touch

- Any `crates/argo-store/src/migrations/vN.rs` file — those are per-version migrations, not in scope.
- `ensure_schema_table` at line 136 — stays in `mod.rs` (boot calls it directly).
- Tests — re-export keeps the call sites stable.

## Acceptance criteria

- [ ] `mod.rs` ≤ 100 LOC after the split.
- [ ] `registry.rs` ≤ 450 LOC.
- [ ] `all_migrations()` is re-exported via `pub(crate) use registry::all_migrations;` from `mod.rs` so all existing call sites keep working without changes.
- [ ] Zero behavior change — same migrations execute in the same order on boot.
- [ ] Migration-chain integration tests (`cargo test -p argo-store migration_reconciliation_tests`) pass — they enumerate the registry; if reordering happens accidentally they'll catch it.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-store` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-store-migrations-mod-split`
- PR title: `store: extract migrations registry sibling (claude-store-migrations-mod-split)`

## Notes for the implementer

- Mechanical. Move the `all_migrations()` function verbatim. Don't try to reorganise the migration list or rename anything.
- The 50+ `mod vN;` lines stay in `mod.rs` — they're 1 line each, can't move them to a sibling without breaking submodule resolution.
- If `Migration` is defined in `mod.rs` itself, that stays put — only the `all_migrations()` function moves.
- The split-prep is `*-split`-suffixed so the pre-claim ceiling check auto-skips.
