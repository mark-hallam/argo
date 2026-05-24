# Round 47 — PR #2184 `fix-skill-worker-overlay-blob-text-join-mismatch` POST-MERGE anti-theater verify (7th recheck round)

**Date**: 2026-05-24 (UTC) · **Round**: 47 · **Pillar**: Skills shared and learned (Claim 4) · runtime overlay-attach data path · P1 data-integrity bug
**Verdict**: 🟢🟢 **POST-MERGE PASS — 7th-consecutive-round watch CLOSED (5 RED → 5 GREEN)**

## Setup

- Round 41/42/43/44/45/46 documented `store_worker_overlay_tests` 2/5 RED on origin/main: `list_for_worker_returns_attached_active_overlays` + `list_for_worker_filters_other_workers` both asserted `left=0 right=1` because production JOIN `s.id = ws.skill_id` compared BLOB `skill.id` to TEXT `worker_skill.skill_id` and silently returned zero matches.
- PR #2184 MERGED 2026-05-24T08:44:58Z by codex conductor as `2a818b18b` (sha). Path A read-side coercion fix.
- Production diff (2 lines): `crates/argo-skill/src/store_lookups.rs:119`
  - Was: `JOIN skill s ON s.id = ws.skill_id`
  - Now: `JOIN skill s ON lower(hex(s.id)) = replace(lower(ws.skill_id), '-', '')`
- Regression test added: `crates/argo-skill/tests/per_worker_overlay_visibility_regression.rs` (+84 LOC).

## Observed

Cold `cargo test -p argo-skill --lib store_worker_overlay_tests::` against `origin/main` HEAD (fresh `CARGO_TARGET_DIR=/tmp/eval-r47-target`):

```
running 5 tests
test store_worker_overlay_tests::list_for_worker_skips_non_active_lifecycle_states ... ok
test store_worker_overlay_tests::list_for_worker_skips_superseded_rows ... ok
test store_worker_overlay_tests::list_for_worker_returns_attached_active_overlays ... ok    ← was RED rounds 41-46
test store_worker_overlay_tests::list_for_worker_filters_other_workers ... ok               ← was RED rounds 41-46
test store_worker_overlay_tests::list_for_worker_returns_empty_when_no_overlays_attached ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 351 filtered out; finished in 0.65s
```

**Both rounds-41-46 RED tests now PASS on a cold build of `origin/main` — verified the 2-line SQL coercion is the real load-bearing fix.**

## Anti-theater check

- Production SQL inspected at `store_lookups.rs:116-124` — coercion is in the JOIN predicate itself; cannot be bypassed by callers or query mode. The previously-zero-match join now finds rows when the BLOB hex equals the TEXT UUID with dashes stripped.
- Tests are the SAME tests that asserted-RED for 6 prior rounds (round 41 confirmed they were not test-flake — production code path was wrong). The assertions did NOT change in #2184 (verified by `git show 2a818b18b` — only `store_lookups.rs` and a NEW test file are touched, the existing `store_worker_overlay_tests` module was unmodified). The tests went from RED → GREEN because production behavior changed, not because the tests were relaxed.
- The new `per_worker_overlay_visibility_regression.rs` test adds an additional end-to-end pin (storage layer through the load path).
- Cross-checked: the test asserts `assert_eq!(overlays.len(), 1)` (real positive assertion) and `worker_b` filter does NOT see `worker_a`'s overlay (real negative assertion). Both meaningful.

## Gap / follow-up

NONE for this scenario. **Claim 4 MVP row 2 e2e regression path can now legitimately return overlays at runtime — the appear-pass-while-returning-zero failure mode (called out by round 44) is closed.**

## Brief candidates surfaced this round

None for this scenario. The companion brief `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (added gate 6 = unskippable cargo test) is STILL on the ledger as BLOCKED (stream file missing) — recommend conductor un-block + author the missing stream file, but this is a separate (P1, gate-hardening) follow-up not a #2184 follow-up.
