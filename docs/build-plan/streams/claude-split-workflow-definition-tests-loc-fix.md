# claude-split-workflow-definition-tests-loc-fix

**Authored against**: origin/main `b0afe9f99` on 2026-05-23
**Owner**: claude (fast-mode eligible — pure mechanical split)
**Priority**: 🔴 P0 — BLOCKS EVERY admin-merge until landed
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-split-workflow-definition-tests-loc-fix`

## Why this is P0

`scripts/conductor-pre-merge-gate.sh` fails on `origin/main` because
`crates/argo-store/src/workflow_definition_tests.rs` is **571 LOC**
(ceiling = 500). The file shipped at this size via PR #2078
(2026-05-23) without enforcement. Every subsequent admin-merge tick
will FAIL the gate, even on PRs that don't touch this file.

CLAUDE.md HARD RULE 6: no file > 500 LOC. CI enforcer runs this in
`bash scripts/conductor-pre-merge-gate.sh`. We cannot ship PRs
through admin-merge while this is red.

## Scope

Split `crates/argo-store/src/workflow_definition_tests.rs` (571 LOC,
16 `#[tokio::test]` functions) into TWO sibling files, both under
ceiling. Suggested cut (natural boundary at line 485, where the
in-tx-rollback cluster begins):

| File | Tests | Approx LOC |
| --- | --- | --- |
| `workflow_definition_tests.rs` (keep) | 13 CRUD + tenant + duplicate + migration + concurrent tests (line 30→485, excludes the 3 tx-rollback tests at the tail) | ~480 |
| `workflow_definition_tx_rollback_tests.rs` (NEW sibling) | 3 in-tx rollback tests (`set_head_in_tx_rolls_back_when_caller_drops_tx`, `archive_in_tx_rolls_back_when_caller_drops_tx`, `rollback_to_in_tx_rolls_back_when_caller_drops_tx`) at lines 485→571 | ~100 |

Both files share the same `fresh_pool()` + `sample_toml()` helpers
(lines 12-28). Two clean options for the helper-sharing:

- **Option A (preferred)** — extract helpers into `pub(crate) mod
  workflow_definition_test_helpers;` under
  `crates/argo-store/src/workflow_definition.rs`, both test modules
  `use super::test_helpers::{fresh_pool, sample_toml};`.
- **Option B (lighter)** — duplicate the 16-line helper block at the
  top of the new file. Zero abstraction surface; ≤16 LOC of code-clone
  is below the "premature abstraction" bar.

Worker picks whichever is mechanically simpler. Either works.

## May touch

- `crates/argo-store/src/workflow_definition_tests.rs` (shrink)
- `crates/argo-store/src/workflow_definition_tx_rollback_tests.rs` (NEW)
- `crates/argo-store/src/workflow_definition.rs` (if Option A — add
  `#[cfg(test)] mod test_helpers;` declaration)
- `crates/argo-store/src/lib.rs` ONLY if either tests module needs to
  be declared there (read first — most likely they're already a
  child mod under `workflow_definition.rs`).

## Must NOT touch

- Production code in `workflow_definition.rs` — pure test-organization
  refactor.
- Any other crate.
- Migration files / schema slot reservations.

## Acceptance

1. `wc -l crates/argo-store/src/workflow_definition*` shows BOTH files
   ≤ 500 LOC.
2. `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` reports
   `[2/5] LOC budget` GREEN (no files over ceiling).
3. `cargo test -p argo-store --lib workflow_definition` runs all 16
   tests with the same pass/fail counts as `origin/main`.
4. `cargo clippy -p argo-store --all-targets -- -D warnings` clean.

## Out of scope (do NOT bundle)

- Renaming any test function (zero churn).
- Adding new tests.
- Refactoring `workflow_definition.rs` production code.
- Adding rustdoc to test functions.

## LOC budget

~20 LOC net (mechanical move + 1-line module declaration). Brief
estimate ~30 LOC including the helper-extraction option.

## Fast-mode eligibility

YES — pure mechanical split with deterministic acceptance. Codex
fast-mode or claude haiku adapter both fine.
