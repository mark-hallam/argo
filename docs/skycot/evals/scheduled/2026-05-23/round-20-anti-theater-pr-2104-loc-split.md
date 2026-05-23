# Round 20 — anti-theater verify #2104 LOC-split is faithful

- **Date**: 2026-05-23
- **Round number**: 20
- **Pillar/Claim**: Cross-cutting — CI hygiene (LOC budget gate)
- **Verdict**: 🟢 PASS (no severity tier — substrate is faithful)

## Setup

`#2104` (`claude-split-workflow-definition-tests-loc-fix`) split the
571-LOC `crates/argo-store/src/workflow_definition_tests.rs` into:

- `workflow_definition_tests.rs` — 479 LOC, 12 CRUD/tenant/migration tests
- `workflow_definition_tx_rollback_tests.rs` — 121 LOC, 3 `_in_tx`
  rollback-on-drop tests

A test-file split is the classic anti-theater risk: a "mechanical
move" can silently drop assertions, rename without updating
callers, or reuse a stale helper. Verified four invariants.

### Verification commands

```bash
# 1. count tests pre-split (HEAD~1 of merge commit 714d6be88)
git show 714d6be88^:crates/argo-store/src/workflow_definition_tests.rs \
  | grep -nE "#\[(tokio::)?test\]"      # → 15 hits

# 2. count tests post-split
grep -nE "#\[(tokio::)?test\]" crates/argo-store/src/workflow_definition_tests.rs \
                               crates/argo-store/src/workflow_definition_tx_rollback_tests.rs
# → 12 + 3 = 15 hits

# 3. byte-diff the 3 moved tests
git show 714d6be88^:crates/argo-store/src/workflow_definition_tests.rs \
  | sed -n '485,571p'    > /tmp/pre.txt
sed -n '35,121p' crates/argo-store/src/workflow_definition_tx_rollback_tests.rs \
                         > /tmp/post.txt
diff /tmp/pre.txt /tmp/post.txt   # → exit 0, byte-identical

# 4. byte-diff the kept helpers (fresh_pool + sample_toml)
git show 714d6be88^:crates/argo-store/src/workflow_definition_tests.rs \
  | sed -n '1,30p'       > /tmp/pre-h.txt
sed -n '1,30p' crates/argo-store/src/workflow_definition_tests.rs \
                         > /tmp/post-h.txt
diff /tmp/pre-h.txt /tmp/post-h.txt    # → exit 0, byte-identical

# 5. run the full suite
cargo test -p argo-store --lib workflow_definition
# → test result: ok. 20 passed; 0 failed; 0 ignored
```

## Observed

| Check | Result |
|---|---|
| Pre-split test count (`#[tokio::test]` lines) | 15 |
| Post-split test count (sum across both files) | 12 + 3 = **15** ✅ |
| Byte-diff: 3 moved test bodies | **identical** (`exit 0`, 87 lines each) ✅ |
| Byte-diff: kept helpers in workflow_definition_tests.rs | **identical** (`exit 0`) ✅ |
| Duplicate helpers in tx_rollback_tests.rs (lines 15-30 vs orig 10-25) | **identical** body, only the docstring + dropped `use ArgoError` differ ✅ |
| `cargo test -p argo-store --lib workflow_definition` | **20 passed; 0 failed; 0 ignored** ✅ |
| Both files under 500 LOC ceiling | 479 + 121 ✅ |

The 3 `_in_tx` rollback tests
(`set_head_in_tx_rolls_back_when_caller_drops_tx`,
`archive_in_tx_rolls_back_when_caller_drops_tx`,
`rollback_to_in_tx_rolls_back_when_caller_drops_tx`) all execute in
the post-split file and pass green.

## Anti-theater check

The genuine anti-theater risks for a "split-only" PR are:

1. **Assertions silently dropped** — would show up as fewer tests or
   shorter test bodies post-split. Both byte-diffs return exit 0 →
   no assertions touched.
2. **Test renamed to skip a known failure** — would show as a name
   change. All 15 test names match across pre/post (eyeballed +
   grep'd).
3. **Helper changed to be more permissive** — would show as a diff
   in `fresh_pool` or `sample_toml`. Both helpers byte-identical to
   pre-split.
4. **Suite count drops because a `mod` declaration was missed** —
   would show in cargo output. 20 passed / 494 filtered matches the
   PR's stated acceptance "20 passed (13 + 4 existing + 3 new)";
   the breakdown labels are slightly off (actual: 12 + 3 + 5
   sibling) but the total is intact.

All four risks ruled out.

## Gap / follow-up brief candidates

None. The split is mechanical, byte-faithful, and verified green on
origin/main. No brief recommendations.

The one observation worth noting (informational, not a finding):
the PR description says "13 CRUD/tenant/duplicate/migration/concurrent"
but the actual count in the kept file is 12. Off-by-one in the
worker's report, no impact on the code. Mentioning so the conductor
isn't surprised if they reconcile counts later.
