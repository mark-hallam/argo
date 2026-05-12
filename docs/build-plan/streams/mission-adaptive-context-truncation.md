# Mission load-bearing wire — Adaptive context truncation (fallback for hard window hits)

**Owner**: claude. **Depends on**: `mission-context-window-aware-compression` MERGED (the compression path is preferred; this is the fallback).
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 8 #248. When LLM compression can't reduce enough (e.g. compression itself fails, or a single turn is too large), we need a deterministic fallback that truncates to fit the window WITHOUT dropping load-bearing structure.

## Scope

NEW `argo-runtime::context_truncate` module providing a `truncate_to_fit(messages, max_tokens, tokenizer)` function:

1. **Priority order** for keeping (highest to lowest):
   - System prompt (never drop)
   - Last user turn (never drop — that's what the model is responding to)
   - Last assistant turn (preserve recent context)
   - Tool-result turns from the last N turns (default 3) — load-bearing for tool-call chains
   - Older user/assistant turns (drop oldest first)
2. **Truncate within a turn** as last resort: if a single turn (e.g. a giant tool output) is > 50% of the window, truncate that turn's content with `[...truncated XXX tokens of tool output...]` marker.
3. **Audit row** per truncation (kind: `runtime.context_truncated`) with `{ pre_token_count, post_token_count, turns_dropped, intra_turn_truncations }`.
4. **Hard-fail** if even after truncation the system prompt + last user turn don't fit: return a typed `ArgoError::ContextWindowExhausted` — caller must handle.

This is NOT in scope: streaming-aware truncation (separate brief — mission-streaming-aware-retry); intelligent picking of which middle-turns to drop (FIFO order in v1); compression of individual turn content (compression brief handles cross-turn).

## Files you may touch

- `crates/argo-runtime/src/context_truncate.rs` (NEW, ≤ 350 LOC)
- `crates/argo-runtime/src/context_truncate_tests.rs` (NEW, sibling, ≤ 350 LOC) — 8 tests
- `crates/argo-runtime/src/lib.rs` — narrowly scoped: register module
- `argo-types::ArgoError` — add `ContextWindowExhausted` variant (or use existing closest variant; verify at claim)
- `docs/skycot/RUNBOOK.md` — append context-truncation cross-reference to the compression section

## Files you must NOT touch

- The compression module (sibling brief — call into it but don't reach into internals)
- The DB representation of messages

## Acceptance criteria

- [ ] Priority order respected: system prompt, last user, last assistant, recent tool-results, older turns dropped FIFO
- [ ] Intra-turn truncation triggers only when a single turn > 50% of window
- [ ] Audit row per truncation
- [ ] `ContextWindowExhausted` typed error when even the minimum can't fit
- [ ] 8 unit tests covering: just-fits / drop-one-turn / drop-many / intra-turn-truncate / system-prompt-too-big-fails / hard-fail-edge / etc.
- [ ] `cargo clippy -p argo-runtime --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-adaptive-context-truncation`
- PR title: `runtime: adaptive context truncation (mission-adaptive-context-truncation)`

## Notes

- This is the deterministic fallback. Compression (separate brief) is the smart-but-LLM-dependent path. When compression fails (rate-limit, refusal, network), truncation is what keeps the agent moving.
- The intra-turn truncation marker `[...truncated XXX tokens of tool output...]` is operator-readable in audit drill-in. Don't silently drop content.
- The hard-fail at the end is intentional. If even the minimum can't fit, the operator deserves to know — quietly accepting a bad response is worse than failing loudly.
