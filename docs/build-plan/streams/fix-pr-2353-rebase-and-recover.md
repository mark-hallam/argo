# fix-pr-2353-rebase-and-recover

**Owner**: claude or codex (fast-mode candidate, ~10 LOC verify + audit).
**Authored against**: origin/main `c35ad17d9` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2353-rebase-and-recover`.
**Source**: evaluator round 84 batch-pre-merge-drain (`docs/skycot/evals/scheduled/2026-05-26/round-84-batch-pre-merge-drain.md`) brief-shape recommendation (P3, "belt-and-suspenders"). Triggered by the note on #2353 (`fix-audit-feed-include-self-hash-in-envelope`): "PR is branched before #2295 (Track P P-02 truth-boundary gate) landed; `git diff main` shows the truth-boundary files as 'removed' but `gh pr diff` confirms the PR itself does NOT touch those files. GitHub's 3-way merge will preserve #2295. Merge-safe."

🟢 **P3 — belt-and-suspenders post-merge audit.** PR #2353 was merged at T10-05 with R84 PRE-MERGE PASS. The evaluator noted a `git diff main` anomaly where the truth-boundary files (added by Track P P-02 #2295) appeared as "removed" in #2353's pre-rebase state. The evaluator concluded GitHub's 3-way merge would preserve them, and merged. This brief is the post-merge verification that the conclusion held.

## Acceptance criteria

- [ ] Run `git log --oneline -10 -- crates/argo-agent-runner/src/turn_helpers_truth_boundary.rs` and confirm:
  - The file exists on current `origin/main`.
  - The HEAD commit of the file is `#2295` (commit `ed0d34785` or its squashed form), NOT something that "removed" or reverted it.
- [ ] Run `git log --oneline -10 -- crates/argo-agent-runner/src/turn_helpers_truth_boundary_tests.rs` — same check.
- [ ] Run `git log --oneline -10 -- crates/argo-capability/src/refusal_registry.rs` — same check.
- [ ] Run `cargo test -p argo-agent-runner truth_boundary` — should show the 11 truth-boundary tests that PR #2295 added.
- [ ] Run `rg "ClaimAllowed|ClaimBlocked|ClaimEscalated" crates/argo-audit/src/event_kind.rs` — confirm all 3 variants still present.
- [ ] If ALL above pass: write a 1-line confirmation comment to this brief's PR (or just commit a docs note in `docs/skycot/audit/post-merge-checks/`) and ledger the brief as MERGED-AS-VERIFIED.
- [ ] If ANY fail: surface immediately to operator + author a fix-* brief to restore the lost code (highly unlikely but the whole point of belt-and-suspenders).

## May-touch

- `docs/skycot/audit/post-merge-checks/2026-05-26-pr-2353-truth-boundary-preserved.md` (NEW — 5-10 line confirmation log)

## Out of scope

- Any code changes — purely audit/verify.

## Worker note

This is the kind of brief that takes ~3 minutes if everything's fine. Falls under operator-locked ops-* `--skip-clippy` lane after evaluator pre-verification (which can be informal since it's a docs-only audit). If everything checks out, just write the confirmation doc and move on.
