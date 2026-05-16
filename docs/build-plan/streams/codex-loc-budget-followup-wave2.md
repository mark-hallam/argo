# codex-loc-budget-followup-wave2 — split next 5 fn-LOC violators (P2 fast-mode)

**Owner**: codex. **Sizing hint**: fast-mode candidate — pure mechanical extraction, no behavior change. **Depends on**: `codex-fn-loc-followup-wave1` MERGED (currently DIRTY #1773).
**Authored against**: origin/main `bf84d768` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-loc-budget-followup-wave2`.
**Source**: PR #1734 body: "running the new script surfaces 205 violators ... these are out-of-scope follow-ups". Wave 1 took the audit-cited top-5; this wave takes the next 5 largest.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — function-LOC budget is a non-negotiable architecture invariant.
- **Repeatability** (#3) — smaller functions are tested + debugged + replaced faster.

## Scope

P2 fast-mode. After Wave 1 (#1773) MERGES, run the violator script + take the next-5 largest **production** functions (skip `_tests.rs`):

```bash
bash scripts/check-fn-loc.sh 2>&1 \
  | grep -v '_tests.rs' \
  | sort -t: -k3 -nr \
  | head -10
```

For each of the top-5: extract sub-functions OR introduce a helper struct holding bag-of-state, so the public function shrinks below 80 LOC. Same discipline as Wave 1.

Wave 1 cited likely candidates: `routes_dispatch_post.rs:136`, `worker_actions.rs:35`, `input.rs:27`. Wave 2 worker picks fresh top-5 against current main, listing them in the PR body before extracting.

**Fast-mode justification:** pure mechanical work, no architectural decisions, no behavior change, no new public API. Speed > depth.

LOC: ~300 net.

## Files you may touch

Depends on top-5 violators at claim time — worker enumerates in PR body. Likely candidates given the codebase's largest production files:

- `crates/argo-cli/src/handlers/gateway/routes_*.rs` (the gateway has many large handlers)
- `crates/argo-cli/src/handlers/worker_actions*.rs`
- `crates/argo-agent-runner/src/turn*.rs`
- `crates/argo-cli/src/handlers/input.rs`
- `crates/argo-cli/src/handlers/boot*.rs`

**Files always touched:**
- Each touched function's sibling test file — extend to assert post-split behavior. ≤40 modified LOC.

## Files you must NOT touch

- The CI gate script (`scripts/check-fn-loc.sh`) — Wave 1 ships it; this brief consumes it.
- Any `_tests.rs` violator — that's a separate brief (`mission-test-fn-loc-budget`) if the operator decides to tighten test-side too.
- Behavior changes — pure extract-and-move only.
- New crates. No.

## Acceptance criteria

- [ ] PR body lists the 5 specific functions split with before/after LOC.
- [ ] All 5 split functions have an entry-point ≤80 LOC.
- [ ] Extracted helpers are each ≤80 LOC.
- [ ] No public API surface change (run `git diff` filtered to `pub fn` / `pub struct` lines — must be zero).
- [ ] No behavior change asserted by existing tests passing without modification.
- [ ] `cargo test -p <touched-crate>` passes for every touched crate.
- [ ] `cargo clippy -p <touched-crate> --all-targets -- -D warnings` clean.
- [ ] `bash scripts/check-fn-loc.sh | grep -E '<one of the 5 file:line>'` returns 0 matches.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-loc-budget-followup-wave2`
- Title: `refactor: split next-5 fn-LOC violators wave 2 (codex-loc-budget-followup-wave2)`
- Worker verification: crate-scoped tests + clippy + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pre-claim check: confirm #1773 MERGED.** Worker should `git log origin/main -- scripts/check-fn-loc.sh` and verify the F-09 script + Wave-1 splits are present.
- **Pick top 5 by absolute delta-to-threshold,** not alphabetical. Largest sources of risk first.
- **Argument list growth is OK.** Don't create a 10-field struct just to flatten args.
- **Helpers are `fn`, not `pub fn`.** Private to module.
- **Don't merge unrelated fixes.** Any TODO / buggy line inside a touched function stays as-is. File a separate brief if needed.
