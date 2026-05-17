# codex-argo-store-schema-v70-test-bumps — mechanical 69→70 bumps in argo-store tests (follow-up from #1822)

**Owner**: codex. **Sizing hint**: fast-mode — pure mechanical sed-style edits.
**Depends on**: none (origin/main has `EXPECTED_SCHEMA_VERSION = 70` since `v70.rs` migration was registered).
**Authored against**: origin/main `b866e8f7` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-argo-store-schema-v70-test-bumps`.
**Source**: PR #1822 "Boundary note" — 8 `argo-store --lib` tests assert
`schema_version == 69` but `EXPECTED_SCHEMA_VERSION = 70` (`crates/argo-store/src/lib.rs:289`).
These tests had not run on origin/main recently because the slot-113 collision
blocked `argo-store` from building (fixed by HOTFIX `096b04b9` 2026-05-17).

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — `cargo test -p argo-store --lib`
  should be green on a clean main. Today it is red on 8 assertions; without
  this fix the test signal is unusable.

## Scope

P2 mechanical. Update every literal `69` assertion of `schema_version` to `70`.

### Sites (verified at `b866e8f7`)

| File | Lines |
|---|---|
| `crates/argo-store/src/lib_tests.rs` | 11 |
| `crates/argo-store/src/approvals_tests.rs` | 58, 73 |
| `crates/argo-store/src/migration_reconciliation_tests.rs` | 15, 122, 179, 228, 253, 330 |

That's 9 sites. (PR #1822 boundary note said "8" — re-check on claim;
`grep -nE 'schema_version.*69' crates/argo-store/src/` is authoritative.)

### Also verify

- `crates/argo-store/src/migrations/slot_validator_tests.rs:5` has
  `let versions = 1..=69;`. If `v70.rs` is in the registered slot list,
  bump the range to `1..=70`. Otherwise leave alone — confirm by
  inspecting `crates/argo-store/src/migrations/registry.rs`.
- Doc-comments in `crates/argo-store/src/outbound_message_query.rs:6` and
  `crates/argo-store/src/tenant_query.rs:339` mention "versioned migration
  runner stops at slot 69". If the runner now stops at 70, update the
  prose. Cosmetic — don't block on it.

LOC: ~20 net (9-12 line edits).

## Files you may touch

- `crates/argo-store/src/lib_tests.rs` — ≤2 modified LOC.
- `crates/argo-store/src/approvals_tests.rs` — ≤4 modified LOC.
- `crates/argo-store/src/migration_reconciliation_tests.rs` — ≤12 modified LOC.
- `crates/argo-store/src/migrations/slot_validator_tests.rs` — ≤2 modified LOC (conditional, see above).
- `crates/argo-store/src/outbound_message_query.rs` — ≤2 modified LOC (cosmetic).
- `crates/argo-store/src/tenant_query.rs` — ≤2 modified LOC (cosmetic).

## Files you must NOT touch

- `EXPECTED_SCHEMA_VERSION` itself (`crates/argo-store/src/lib.rs:289`) — already
  correct at 70.
- Migration source files (`crates/argo-store/migrations/*.sql` or
  `crates/argo-store/src/migrations/v*.rs`) — read-only consumer.
- Tests asserting non-final intermediate schema versions (e.g.
  `migration_reconciliation_tests.rs:118` asserts 11, `:257` asserts 16,
  `:334` asserts 22). These are step-wise assertions partway through a
  migration — DO NOT bump them.

## Acceptance criteria

- [ ] `cargo test -p argo-store --lib` clean (was 8 failures pre-fix).
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Public API surface unchanged (verify with `git diff --stat` — should be
      pure test edits).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-argo-store-schema-v70-test-bumps`
- Title: `test(store): bump schema_version assertions 69→70 (post-slot-113 hotfix unblock) (codex-argo-store-schema-v70-test-bumps)`
- Worker verification: crate-scoped clippy + tests per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pure mechanical.** No new tests. No behavior change. Single-shot bump.
- **Step-wise assertions stay put.** Tests that walk through migrations and
  assert intermediate versions (11, 16, 22) are partway-through checkpoints —
  the final assertion is the one that needs bumping.
- **Slot-113 hotfix is the unblocker** — `096b04b9` renamed the duplicate to
  slot 114 so `argo-store/build.rs` no longer panics on duplicate slot
  detection; tests now run and surface the stale 69 assertions. This brief
  closes that follow-up.
