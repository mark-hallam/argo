# fix-skill-test-claim-4-row-5-6-workspace-clippy-lints

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `91fcd8157` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skill-test-claim-4-row-5-6-workspace-clippy-lints`.

🔴 **P1 GATE-UNBLOCK (de-facto P0 — blocking 2 IN_REVIEW PRs).** Workspace clippy
RED on `origin/main` across two `crates/argo-skill/tests/*.rs` files. Every
admin-merge that does NOT pass `--skip-clippy` is blocked until this lands.
Round-44 evaluator P0'd `--skip-clippy` as a structurally masking pattern
(it carried a `store_worker_overlay_tests` 2/5 RED silently across 6 PRs);
the right move is to fix the drift, not skip the gate.

**Currently blocking**: PR #2131 (mission-orchestrator-cross-tenant-handoff-audit-kind,
CLEAN code review, gate FAIL tick-132) and PR #2171 (eval-argo-sandbox-deferred-status-pin,
CLEAN, gate would also FAIL because workspace clippy is workspace-wide).
SUPERSEDES `fix-claim-4-mvp-row-5-clippy-drift` (sibling brief covering only
row-5; this brief covers both row-5 and row-6 in a single PR).

## Why

`cargo clippy -p argo-skill --tests --quiet` against origin/main `91fcd8157`
surfaces 4 errors (under `-D warnings`):

1. `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:10:5` —
   `doc_lazy_continuation`. Line 9 begins with `//! + \`cli_skill_review_cmds.rs\`.`;
   clippy reads the `+` as a markdown list-item bullet and expects continuation
   lines 10-12 indented under it.

2. `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:11:5` —
   same `doc_lazy_continuation`, continuation line.

3. `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:12:5` —
   same `doc_lazy_continuation`, continuation line `//! on.`.

4. `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs:105:5` —
   `assertions_on_constants`. `assert!(DEFAULT_SUCCESS_THRESHOLD < 0.9);`
   compares a `const f64` against a literal; clippy flags this as a no-op at
   runtime. Fix: replace with `const _: () = assert!(DEFAULT_SUCCESS_THRESHOLD < 0.9);`
   (compile-time assertion) — preserves the invariant and silences the lint.

5. `crates/argo-skill/tests/claim_4_mvp_row_6_operator_approve_emits_audit.rs:123:15` —
   `unnecessary_lazy_evaluations`. `.or_else(|| None)` is a no-op; clippy
   suggests `.or(None)`. (Strictly, since the closure always returns `None`,
   the whole `.or_else(|| None)` can be deleted — the chain already
   short-circuits on the `find` result. But the safe mechanical fix is
   `.or(None)`.)

Both files landed at commit `95770b22b` (evaluator round 31) and have been
flagged out-of-may-touch by every subsequent PR (#2149 / #2152 boundary notes).

## Scope

1. Edit `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs` and
   `crates/argo-skill/tests/claim_4_mvp_row_6_operator_approve_emits_audit.rs`
   only.
2. Resolve the 4 clippy errors above with the minimal change that preserves
   intent (the doc-comment narrative + the threshold invariant + the Option
   chain semantics).
3. Verify per-crate clippy is clean.
4. Verify per-crate tests still pass.
5. Verify `scripts/workspace-clippy-fast-fail.sh` reaches GREEN end-to-end. If
   it surfaces a third pre-existing drift in another `crates/argo-skill/tests/*.rs`
   file, this brief widens to cover the immediate cluster — but only
   `crates/argo-skill/tests/*.rs`. Cross-crate drift requires its own brief.

## May touch

- `crates/argo-skill/tests/claim_4_mvp_row_5_pending_review_queue.rs`
- `crates/argo-skill/tests/claim_4_mvp_row_6_operator_approve_emits_audit.rs`

## Acceptance

- [ ] 4 clippy errors above resolved.
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-skill --test claim_4_mvp_row_5_pending_review_queue` 5/5 green.
- [ ] `cargo test -p argo-skill --test claim_4_mvp_row_6_operator_approve_emits_audit` green.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` GREEN end-to-end (or surface the next gate failure if a third pre-existing drift exists).
- [ ] `scripts/conductor-pre-merge-gate.sh` reaches GREEN end-to-end on a
      synthetic PR (no `--skip-clippy`) — verifies the gate-unblock thesis.
- [ ] File LOC unchanged or smaller; HARD RULE 1 (≤500 LOC) preserved.

## Fast-mode candidate

Yes — mechanical rule-driven changes, ~5-15 LOC delta total, no architectural judgement.

## Anti-theater note

The fix is purely lint-suppression-via-correctness — DO NOT change test
behavior, assertion targets, or doc-narrative meaning. The doc-continuation
indent fix is whitespace-only; the `const _: () =` wrapper preserves the
compile-time check; the `.or(None)` is byte-equivalent semantics. If the
mechanical fix changes any observed test outcome, stop and re-evaluate —
that's a sign the lint was load-bearing.
