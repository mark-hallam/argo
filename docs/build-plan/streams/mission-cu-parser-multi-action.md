# Mission load-bearing wire — CU output parser handles multi-action turns

**Owner**: claude. **Depends on**: none (pure executor-internal change).
**Authored against**: origin/main `c91e37e5` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-parser-multi-action`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 item #94 ("Fix CU output parser to handle multi-action turns"). The parser at `anthropic_parse.rs:12` documents the bug in its own comment: *"Walk the content blocks; the first tool_use wins."* When Claude emits two `tool_use` blocks in one turn (e.g. `mouse_move` then `left_click`), the second is silently dropped, the executor takes one action, screenshots, asks for the next action, and pays an extra round-trip. Mission Claim 5 ("real-world execution") suffers a measurable latency penalty per dropped action; the fix is surgical.

## Scope

Widen `parse_response` from `Result<CuStepResponse, CuError>` to `Result<Vec<CuStepResponse>, CuError>` and walk **all** `tool_use` blocks in the response, not just the first. The single call site in `anthropic.rs:82` becomes the only mutation point outside the parser; downstream the executor loop dispatches the returned Vec sequentially before re-screenshotting.

If the response carries no `tool_use` blocks but DOES carry a `stop_success` / `stop_failure` text signal, return a single-element Vec carrying the stop response (preserves today's behavior for terminal turns). Empty Vec is an error condition (today's `"anthropic response carried no tool_use or recognised stop signal"` raise) — preserve that exact error message.

This is NOT in scope: speculative parallelism (the actions still run sequentially); changing the executor's turn cadence beyond "process N actions per response then re-screenshot"; modifying `CuStepResponse` itself; supporting tool_uses with explicit dependencies between them.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use/src/anthropic_parse.rs` — change `parse_response` signature; collect Vec of CuStepResponse from all tool_use blocks; preserve stop-signal fallback as 1-element Vec.
- `crates/argo-executor-computer-use/src/anthropic.rs` — update the single call site at line 82 to receive Vec.
- `crates/argo-executor-computer-use/src/anthropic_tests.rs` — update existing 8 tests to assert against `result[0]` (length 1 for single-action responses); add 4 NEW tests: two-tool-uses-yield-vec-of-two; three-tool-uses-yield-vec-of-three; one-tool-use-then-stop-success-yields-two-element-vec-with-stop-last; empty-tool-uses-with-stop-text-yields-one-element-vec.
- `crates/argo-executor-computer-use/src/executor.rs` — narrowly scoped: extend the dispatch loop to iterate over the Vec instead of dispatching a single response. Each action runs to completion before the next; screenshot once after the Vec is exhausted. NO change to inter-turn loop structure.
- `crates/argo-executor-computer-use/src/executor_types.rs` — narrowly scoped: ZERO edits unless `parse_response` callers are re-exported through here (verify in first 5 minutes; if so, narrow re-export adjustment only).

## Files you must NOT touch

- `crates/argo-executor-computer-use/src/learn.rs` / `learn_tests.rs` — feedback loop is independent; don't touch
- `crates/argo-executor-computer-use/src/budget.rs` — token-budget accounting is independent
- `crates/argo-executor-computer-use/src/pool.rs` — sandbox pool unchanged
- `crates/argo-executor-computer-use-macos/**` — native macos executor is a separate consumer
- Any non-executor crate

## Acceptance criteria

- [ ] `parse_response` signature is `Result<Vec<CuStepResponse>, CuError>`
- [ ] All `tool_use` blocks in the response are collected into the Vec in document order
- [ ] When response has tool_uses + stop_success/failure text, the stop response is appended LAST as the final Vec entry (preserves "act then stop" semantics)
- [ ] When response has neither tool_use nor stop signal, returns the existing `"anthropic response carried no tool_use or recognised stop signal"` error verbatim (no message change)
- [ ] Empty Vec is impossible — every Ok return has at least one entry
- [ ] All 8 existing tests in `anthropic_tests.rs` updated to assert `result[0]` and pass
- [ ] 4 new tests added: two-actions; three-actions; act-then-stop; stop-only
- [ ] Executor dispatch loop iterates the Vec sequentially before re-screenshotting (no parallelism)
- [ ] `cargo build -p argo-executor-computer-use` clean
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-parser-multi-action`
- PR title: `cu-executor: parse_response returns Vec for multi-action turns (mission-cu-parser-multi-action)`

## Notes for the implementer

- Today's parser comment ("the first tool_use wins") is the canonical bug marker. Update or delete that comment as part of the fix.
- The dispatch-loop change in `executor.rs` is small: replace `let resp = parse_response(...)?; dispatch(resp).await?;` with `for resp in parse_response(...)? { dispatch(resp).await?; }`. If `dispatch` is named differently (verify in first 5 minutes via `rg`), adapt accordingly.
- Order matters: tool_use[0] must dispatch before tool_use[1]. Test fixture should assert ordering by including a `mouse_move` followed by a `left_click` — if they swap, the click misses.
- If `dispatch` returns an error mid-Vec, abort the remaining actions and bubble up — match today's fail-fast semantics.
- `CuStepResponse` shape is intentionally unchanged. If a downstream consumer depends on receiving exactly one response per `parse_response` call (e.g. via a channel), wrap the single-action consumer in a Vec-iterating helper rather than reverting the signature.
