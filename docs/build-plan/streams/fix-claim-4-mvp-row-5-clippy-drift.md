# fix-claim-4-mvp-row-5-clippy-drift

**Owner**: claude or codex.
**Depends on**: none.
**Authored against**: origin/main `963981405` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-claim-4-mvp-row-5-clippy-drift`.

🔴 **P0 GATE-UNBLOCK.** Workspace clippy currently RED on `origin/main` in `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs` — 4 errors block `scripts/conductor-pre-merge-gate.sh` from passing. Every admin-merge that does NOT pass `--skip-clippy` is blocked until this lands. Round-44 has already P0'd `--skip-clippy` as a masking pattern; the right move is to fix the drift, not skip the gate.

## Why

`tick-132` pre-merge gate (PR #2131) failed gate 5/5 with:

```
error: doc list item without indentation
  --> crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:10:5
error: doc list item without indentation
  --> crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:11:5
error: doc list item without indentation
  --> crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:12:5
error: this assertion has a constant value
   --> crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:105:5
```

File landed at commit `95770b22b` (evaluator round 31). Two distinct lints:

1. **Doc list item drift (lines 10-12)** — line 9 begins with `//! + \`cli_skill_review_cmds.rs\`.`. Clippy reads the `+` as a markdown list-item bullet; the continuation lines 10-12 should be indented under it. Fix: either escape the `+` (`\+`), reword (`// also references \`cli_skill_review_cmds.rs\``), or indent lines 10-12 with two spaces under the `+` marker.

2. **Constant assertion (line 105)** — `assert!(DEFAULT_SUCCESS_THRESHOLD < 0.9);` compares a `const f64` against a literal; clippy flags this as a no-op at runtime. Fix: replace with `const _: () = assert!(DEFAULT_SUCCESS_THRESHOLD < 0.9);` (compile-time assertion) OR remove (the invariant is enforced by source review).

## Scope

1. Edit `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs` only.
2. Resolve the 4 clippy errors with the minimal change that preserves intent (the doc-comment narrative and the threshold-vs-test-input invariant).
3. Verify: `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
4. Verify: `cargo test -p argo-skill --test claim_4_mvp_row_5_pending_review_queue` still passes (5 tests).
5. Verify: `bash scripts/workspace-clippy-fast-fail.sh` reaches GREEN end-to-end (or surface the next gate failure if a second pre-existing drift exists; recommend bundle-fix in the same PR if 1-2 more sibling crates trip).

## May touch

- `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs`

If `workspace-clippy-fast-fail.sh` surfaces a second pre-existing drift in another `crates/argo-skill/tests/*.rs` file, this brief widens to cover the immediate cluster — but only `crates/argo-skill/tests/*.rs`. Cross-crate drift requires its own brief.

## Acceptance

- [ ] 4 clippy errors above resolved.
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-skill --test claim_4_mvp_row_5_pending_review_queue` still 5/5 green.
- [ ] `scripts/conductor-pre-merge-gate.sh` reaches GREEN end-to-end on a synthetic PR (no `--skip-clippy`).
- [ ] File LOC unchanged or smaller; HARD RULE 1 (≤500 LOC) preserved.

## Fast-mode candidate

Yes — mechanical rule-driven changes, ~5-15 LOC delta, no architectural judgement.
