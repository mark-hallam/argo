# claude-split-worker-actions-auto-approval — drop worker_actions_auto_approval.rs below 420 LOC

**Owner**: claude. **Sizing hint**: fast-mode — pure mechanical extraction.
**Depends on**: none.
**Authored against**: origin/main `e4257c58` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-split-worker-actions-auto-approval`.
**Source**: 2026-05-18 audit P1 finding #6. `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` drifted from 490 LOC to **504 LOC** (HARD RULE 6 violation: >500 ceiling) when #1819 added `ApprovalAsked`/`ApprovalReplied` audit variants threaded through the auto-approval policy logic. Currently failing the conductor pre-merge gate from #e4257c58 onward.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — main must be green on the LOC invariant; the pre-merge gate blocks subsequent merges as long as this file is over.
- **Repeatability** (#3) — same mechanical extraction pattern as #1808 + `codex-split-files-450-500-wave`.

## Scope

P1 fast-mode. Pure mechanical extraction to drop the file to ≤420 LOC (80 LOC headroom for future audit-variant additions).

### Strategy

Extract 2 sibling modules:

1. **`worker_actions_auto_approval_gates.rs`** — `apply_effective_gates` + helpers (typical 80–120 LOC). Re-export the public surface, keep callers compiling.
2. **`worker_actions_auto_approval_quiet_hours.rs`** — `quiet_hours_check` + the date/timezone helpers (typical 40–80 LOC).

If those two extractions don't drop the parent file below 420 LOC, also extract the `ApprovalReplied`/`ApprovalAsked` audit-emit helpers added by #1819 into `worker_actions_auto_approval_audit.rs`.

### Discipline

- **No behavior change.** Pure mechanical move.
- **`pub(super)` or `pub(crate)` for extracted helpers** — never expand public surface.
- **Each new sibling ≤80 LOC fn-budget.**
- **Tests stay green without modification** (or with mechanical import-path updates only).

LOC: ~150 net (lines move between files; net additions for `mod` declarations).

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` — net trim. ≤80 modified LOC.
- `crates/argo-cli/src/handlers/worker_actions_auto_approval_gates.rs` (NEW). ≤200 LOC.
- `crates/argo-cli/src/handlers/worker_actions_auto_approval_quiet_hours.rs` (NEW). ≤150 LOC.
- `crates/argo-cli/src/handlers/worker_actions_auto_approval_audit.rs` (NEW, OPTIONAL). ≤120 LOC.
- `crates/argo-cli/src/handlers/mod.rs` — module declarations. ≤4 modified LOC.
- Sibling test files — import-path updates if helpers moved. ≤10 modified LOC.

## Files you must NOT touch

- Behavior. Pure mechanical extraction.
- Public API surface — `pub fn` / `pub struct` declarations stay where they are.
- The CI loc-budget workflow — consume only.
- Other files in the 450-499 band (covered by `codex-split-files-450-500-wave`).

## Acceptance criteria

- [ ] `wc -l crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` shows ≤420.
- [ ] `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` exits 0 on `worker_actions_auto_approval.rs`.
- [ ] All extracted helpers ≤80 LOC.
- [ ] Public API surface unchanged (verify with `git diff --stat` filtered on `pub fn`/`pub struct`/`pub enum`).
- [ ] No behavior change asserted by existing tests passing without functional modification.
- [ ] `cargo test -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-split-worker-actions-auto-approval`
- Title: `refactor: split worker_actions_auto_approval.rs below 420 LOC (claude-split-worker-actions-auto-approval)`
- Worker verification: crate-scoped clippy + tests + loc-budget script per SWARM_PROTOCOL.md.

## Hidden constraints

- **Closest-to-ceiling first.** This file is at 504 (over 500). Other 450-499 band files are covered by `codex-split-files-450-500-wave`.
- **`mod.rs` is the right place for module declarations.** Each new sibling needs `pub(super) mod worker_actions_auto_approval_<area>;`.
- **The new `ApprovalAsked`/`ApprovalReplied` audit emission from #1819 is the most-likely extraction target** if the gates + quiet-hours split alone isn't enough.
- **Closes 2026-05-18 audit P1 #6.** Unblocks the conductor pre-merge gate from gating every subsequent merge on this stale drift.
