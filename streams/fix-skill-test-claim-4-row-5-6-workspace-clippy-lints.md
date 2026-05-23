# fix-skill-test-claim-4-row-5-6-workspace-clippy-lints

**Priority**: P1 (workspace-clippy gate-5 unblocker — second tail of the same chain that #2152 closed)
**Owner**: claude (fast-mode candidate)
**Type**: Mechanical lint fix, tests-only
**Size**: ~5-15 LOC across 2 files
**Authored against**: origin/main `d6f706638` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skill-test-claim-4-row-5-6-workspace-clippy-lints`

## Why

PR #2152 closed the `cron_duty_wire_closure_test.rs` half of workspace-clippy gate-5,
but the gate is **still red** on two `argo-skill` test files. Confirmed live on
`origin/main` `d6f706638` via `cargo clippy -p argo-skill --tests`:

```
warning: doc list item missing indentation
  --> crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:12:5

warning: this assertion has a constant value
  --> crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:105:5

warning: unnecessary closure used to substitute value for `Option::None`
  --> crates/argo-skill/tests/claim_4_mvp_row_6_operator_approve_emits_audit.rs:123:15
```

PR #2152's boundary note flagged this; PR #2149's boundary note flagged it
too. Two consecutive worker-PRs surfaced the same lints as out-of-may-touch.
Until these clear, every conductor tick has to `pre-merge-gate --skip-clippy`,
and any fresh `argo-skill` PR fails its own pre-push clippy gate. This brief
is the single, mechanical fix that finally unblocks gate-5.

## What

Three trivial lint fixes:

1. **`crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:12`** —
   `doc_lazy_continuation`. The `//! on.` continuation line at the end of the
   module docstring needs two extra spaces of indent so clippy recognises it
   as a continuation, not a new paragraph: change `//! on.` → `//!   on.`.

2. **`crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:105`** —
   `assertions_on_constants`. The line currently reads:
   ```rust
   assert!(DEFAULT_SUCCESS_THRESHOLD < 0.9);
   ```
   Both sides are compile-time constants, so clippy wants this in a `const`
   block to make the compile-time-ness explicit:
   ```rust
   const { assert!(DEFAULT_SUCCESS_THRESHOLD < 0.9) };
   ```

3. **`crates/argo-skill/tests/claim_4_mvp_row_6_operator_approve_emits_audit.rs:126`** —
   `unnecessary_lazy_evaluations`. The line currently reads:
   ```rust
   .or_else(|| None)
   ```
   The closure body is a unit literal, so the lazy form is pure overhead. Use
   the eager form:
   ```rust
   .or(None)
   ```
   (`Option::or(None)` is a no-op; the entire `.or_else(|| None)` line could
   also be deleted without changing behaviour. The brief leaves the choice to
   the worker — prefer the smaller diff.)

## Acceptance

- `cargo clippy -p argo-skill --tests -- -D warnings` exits 0 on the fix branch.
- `cargo test -p argo-skill --tests` still passes (the lint fixes are
  behaviour-preserving — the test logic is unchanged).
- `bash scripts/conductor-pre-merge-gate.sh` (no `--skip-clippy`) gate-5
  passes on the fix branch (subject to any unrelated pre-existing breaches in
  other crates not in may-touch).

## May-touch

- `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs`
- `crates/argo-skill/tests/claim_4_mvp_row_6_operator_approve_emits_audit.rs`

**Out of may-touch**: any production code, any other test file. If you find
additional clippy warnings in `argo-skill` not listed above, that is a
separate brief — do not widen scope.

## Parallel surfaces

`rg 'DEFAULT_SUCCESS_THRESHOLD'` returns one definition + one assertion site
(the assertion being fixed); no parallel definitions to worry about.

`rg 'or_else\(\|\| None\)' crates/` returns one match (the line being
fixed); no parallel call-sites.

## Wire

These files are integration tests; they are already wired into
`cargo test -p argo-skill` via the `tests/` directory convention. No further
wire-up needed.

## Sensible deviations

- If a worker hits a fourth lint in either file (added between brief
  authoring and claim), fix it too rather than leaving the gate-5 chain
  half-closed. Document in the PR body.
- If `const { assert!(...) }` is not yet available on the workspace's MSRV
  (Rust 1.75), use `#[allow(clippy::assertions_on_constants)]` with a
  one-line comment pointing at this brief; do not change the workspace MSRV.
