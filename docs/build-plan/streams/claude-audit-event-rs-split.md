# claude-audit-event-rs-split

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `862d45a3b` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-audit-event-rs-split`.

🟡 **P2 LOC-CEILING UNBLOCK.** Source: PR-review agent (tick-38) + repeated pre-merge gate failures. `crates/argo-audit/src/event.rs` is at **579 LOC** on `origin/main`, persistently over the 500-LOC HARD RULE 6 ceiling. The pre-merge gate fails on this every tick; conductors merge through it because it's pre-existing, but the failing gate hides real new regressions. Split now to close the loop.

## Scope

Extract the `AuditKind` enum + its `Display` / `FromStr` / `label()` impls + the variant-count-tracking test out of `event.rs` into a sibling file `crates/argo-audit/src/event_kind.rs`. The split is purely structural — no semantic change to the public API. Re-export `AuditKind` from `event.rs` via `pub use event_kind::AuditKind;` so callsites across the workspace continue to compile unchanged.

After the split:
- `event.rs` should drop below 500 LOC (current 579 LOC minus AuditKind enum + impls ≈ 80-120 LOC removed).
- `event_kind.rs` should be ~100-150 LOC and well under the ceiling.
- All existing tests pass without modification.
- `pre-merge-gate.sh` (LOC budget gate) goes green for `argo-audit`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-audit/src/event.rs` — remove the `AuditKind` enum + impls; add `pub use event_kind::AuditKind;` re-export.
- `NEW: crates/argo-audit/src/event_kind.rs` — the extracted enum + impls + variant-count test.
- `crates/argo-audit/src/lib.rs` — narrowly scoped: add `pub mod event_kind;` line (only if needed for the re-export to work; verify before editing).

## Files you must NOT touch

- Any callsite of `AuditKind` outside `argo-audit` — the re-export keeps callsites compiling.
- Any other file in `crates/argo-audit/src/`.
- The `event.rs` `Event` struct itself.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] `crates/argo-audit/src/event.rs` is under 500 LOC after the split.
- [ ] `crates/argo-audit/src/event_kind.rs` exists, holds the `AuditKind` enum + all impls (`Display`, `FromStr`, `label`, any test that pins the variant count).
- [ ] `crates/argo-audit/src/event.rs` re-exports `AuditKind` via `pub use event_kind::AuditKind;`.
- [ ] `cargo clippy -p argo-audit --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-audit` passes (incl. the variant-count test which lives in event_kind.rs after the move).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (final sanity — confirms no callsite broke).
- [ ] `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` exits 0 (LOC budget gate goes green on argo-audit).
- [ ] No semantic change — every existing test passes without modification (except the variant-count test which only moves).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-audit-event-rs-split`
- PR title: `audit: extract AuditKind into event_kind.rs (claude-audit-event-rs-split)`

## Notes for the implementer

- ~150-200 LOC moved (pure relocation). No new logic.
- The `AuditKind` enum has ~80 variants and a `label()` method that returns a stable string. Both `Display` and `FromStr` impls round-trip on the label. Take the enum, take all of `impl AuditKind`, take the `impl Display` and `impl FromStr` blocks. Leave the `Event` struct and its impls in `event.rs`.
- The variant-count test (likely named `all_kind_count_tracks_declared_variants` or similar — verify before editing) goes with the enum into `event_kind.rs`. Update its module path if the test references siblings.
- Watch out for visibility: if any impl block is `pub(crate)` for a reason, preserve that exact visibility.
- This brief is the proper fix for an issue PR #2010 + #2021 patched around. The pre-merge gate has been failing on `event.rs > 500 LOC` for multiple ticks; landing this brief means the gate gives an honest signal again.
