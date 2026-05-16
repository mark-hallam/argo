# codex-fn-loc-followup-wave1 — split top-5 fn LOC violators surfaced by F-09 (P2)

**Owner**: codex. **Depends on**: `codex-fn-loc-gate-script` MERGED (#1734) — script + the violator list both exist.
**Authored against**: origin/main `223d3fab` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-fn-loc-followup-wave1`.
**Source**: PR #1734 body: "running the new script surfaces 205 violators (audit-cited samples all present: routes_dispatch_post.rs:136, worker_actions.rs:35, input.rs:27); these are out-of-scope follow-ups."

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — function-LOC budget is an architectural invariant (CLAUDE.md line 57: "No fn > 80 LOC").
- **Repeatability** (#3) — small functions are debugged and tested faster.
- **Trust the worker's earlier verification** — `check-fn-loc.sh` is already in CI as informational (not blocking) post-F-09; this brief converts the top-5 cited violators to compliant.

## Scope

P2. Run the F-09 script + pick the top-5 longest functions in the production tree (skip `#[cfg(test)]` bodies):

```bash
bash scripts/check-fn-loc.sh 2>&1 | grep -v '_tests.rs' | sort -t: -k3 -nr | head -10
```

For each of the top-5: extract sub-functions OR introduce a helper struct that holds the bag-of-state, so the public function shrinks below 80 LOC. The audit-cited samples are a starting point: `routes_dispatch_post.rs:136`, `worker_actions.rs:35`, `input.rs:27`.

**Discipline:**
- No behavioral change. Pure mechanical extraction.
- Each extracted helper gets a one-line `///` comment naming its responsibility.
- No new public APIs unless the original function was public — extractions are private.
- No early-return refactors; preserve original error-propagation order.

LOC: ~300 (net; mostly moving code into helpers).

## Files you may touch

Depends on top-5 violators — actual paths picked after running the script. Likely candidates (per audit citations):

- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — split the dispatch entry-point. ≤120 modified LOC.
- `crates/argo-cli/src/handlers/worker_actions.rs` — split the action execution body. ≤120 modified LOC.
- `crates/argo-cli/src/handlers/input.rs` — split the input parser. ≤80 modified LOC.

**OR** any other 2 functions if the script surfaces larger ones than the audit citations. Worker decides at claim time; record the chosen 5 in the PR body.

**Files always touched:**
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_tests.rs` or sibling test files for the touched functions — extend test coverage to assert post-split behavior still passes. ≤40 modified LOC.

## Files you must NOT touch

- The CI gate script itself (`scripts/check-fn-loc.sh`) — that's F-09; this brief consumes it, doesn't modify.
- Any `*_tests.rs` file's `#[cfg(test)]` functions to reduce them just to clear the lint — test functions stay readable per F-09 acceptance ("uniform 80-LOC rule, tests NOT exempt"), but this brief is production-tree only. Test-tree follow-up is separate.
- New crates. No.

## Acceptance criteria

- [ ] PR body lists the 5 specific functions split, with before/after LOC.
- [ ] All 5 split functions have a thin entry-point ≤80 LOC.
- [ ] Helper functions extracted are each ≤80 LOC.
- [ ] No public API surface change. (Run `git diff --stat` of `pub fn` / `pub struct` — must be zero.)
- [ ] No behavior change asserted by existing tests still passing without modification (or with mechanical-only modification — argument-passing follow-throughs).
- [ ] `cargo test -p <touched-crate>` passes for every touched crate.
- [ ] `cargo clippy -p <touched-crate> --all-targets -- -D warnings` clean.
- [ ] `bash scripts/check-fn-loc.sh | grep -E '<one of the 5 file:line>'` returns 0 matches (the 5 violators are gone).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-fn-loc-followup-wave1`
- Title: `refactor: split top-5 fn-LOC violators (codex-fn-loc-followup-wave1)`
- Worker verification: crate-scoped tests + clippy + `cargo clippy --workspace` per SWARM_PROTOCOL.md.

## Hidden constraints

- **Extract, don't inline.** The big function probably has a logical structure already — a "validate input" phase, a "compute" phase, a "format response" phase. Extract along those phase boundaries, not arbitrary 80-line chunks.
- **Helper visibility.** Extracted helpers are `fn`, not `pub fn`. Keep them private to the module.
- **Argument list growth is OK.** Passing 6 args to a helper is fine. Don't create a 10-field struct just to flatten an arg list — that's worse.
- **Don't merge multiple unrelated changes.** If the function had a TODO or a buggy line inside, leave it. This brief is rename + extract only. File bugs as separate briefs.
- **Reserve slot picks** — worker should pick the 5 violators with the **biggest delta to threshold**, not the easiest ones. The point is removing the largest sources of risk.
