# claude-rebase-pr-1790 — recover #1790 codex-fn-loc-followup-wave1

**Owner**: claude. **Recovery target**: PR [#1790](https://github.com/mark-hallam/Argo-Agent/pull/1790). **Original brief**: `streams/codex-fn-loc-followup-wave1.md`.
**Authored against**: origin/main `da578d55` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1790`.
**Priority**: P2 — blocks `codex-loc-budget-followup-wave2`.

## Why this brief exists

PR #1790 was opened 2026-05-16 by a claude worker as the fresh-start successor to closed #1773. It verified clean at open time: per-crate clippy clean + workspace clippy clean + 1857 tests passed (4 KNOWN_FLAKES documented). The triage agent (tick-1, 2026-05-19) classified it MERGE-NOW-URGENT.

Conductor attempted admin-merge in tick-2 (2026-05-19); `gh api repos/.../pulls/1790` reports `mergeable=false`, `state=dirty`. Three days of upstream traffic since 2026-05-16 have introduced conflicts in the high-traffic files this PR extracts from (`worker_actions.rs`, `dispatch_post.rs`, `turn_helpers.rs`, `boot.rs`, `input.rs`).

This brief drives the rebase. Once merged, `codex-loc-budget-followup-wave2` (BLOCKED on this row) unblocks.

## Acceptance: pick ONE recovery path

### Path A — rebase (preferred)

1. `git fetch origin && git checkout claude-codex-fn-loc-followup-wave1`
2. `git rebase origin/main`. Expected conflicts in the 5 extraction targets:
   - `crates/argo-cli/src/handlers/worker_actions.rs` (run fn 425→12)
   - `crates/argo-cli/src/handlers/gateway/boot.rs` (start fn 360→20)
   - `crates/argo-agent-runner/src/turn_helpers.rs` (run_turn_with_tools 334→51)
   - `crates/argo-tui/src/input.rs` (run fn 330→35)
   - `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` (dispatch_post_inner 325→6)
3. Resolution strategy: this PR's extracted helpers should win for the structural shape; for any new logic introduced by upstream merges (e.g. new flags, new audit calls), port those into the new helper files this PR creates.
4. Verification: `bash scripts/check-fn-loc.sh` should report all 5 targeted functions ≤ 80 LOC; per-crate clippy clean for argo-cli + argo-agent-runner + argo-tui; full workspace-clippy-fast-fail clean.
5. `git push --force-with-lease origin claude-codex-fn-loc-followup-wave1`.
6. Comment on #1790: "✅ rebased + force-pushed onto origin/main `da578d55`."

### Path B — fresh start

If Path A's rebase surface is too wide (>5 files of merge resolution beyond the targeted extractions), close #1790 SUPERSEDED and open a fresh `codex-fn-loc-followup-wave1-v2` PR that re-applies the 5 extractions against current main. Note that `check-fn-loc.sh` against current main may report a slightly different top-5 list — extract whatever the script reports today.

**Path A is preferred** because the PR has 1857 tests passing — that verification is preserved across rebase. A fresh start re-introduces test-run risk.

## Acceptance criteria

- [ ] PR #1790 ends MERGEABLE / non-DIRTY against current main.
- [ ] `bash scripts/check-fn-loc.sh` reports all 5 originally-targeted functions ≤ 80 LOC (or, on Path B, today's top-5 ≤ 80 LOC).
- [ ] `cargo clippy -p argo-cli -p argo-agent-runner -p argo-tui --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-agent-runner` clean (KNOWN_FLAKES excepted).
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] BUILD_STATUS.md row `codex-fn-loc-followup-wave1` flipped to IN_REVIEW with PR #1790 (note: current ledger entry incorrectly carries status=DROPPED + PR=#1773; the conductor will fix that flip atomically with this row's status update).
