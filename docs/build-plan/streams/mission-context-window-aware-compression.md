# Mission load-bearing wire — Context-window-aware compression

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 8 #247 (Amendment §5.8). Long-running sessions blow past model context windows. Today we either truncate naively (drop oldest) or fail. Add an LLM-aware compression pass that summarizes older turns while preserving load-bearing structure.

## Scope

NEW `argo-runtime::context_compress` module:

1. **Trigger** — when token-count approaches the per-model window threshold (e.g. 80% of `model.context_window_tokens`). Counted via the model's tokenizer (cl100k_base / o200k for OpenAI, claude tokenizer for Anthropic).
2. **Strategy** — keep the system prompt, the last N turns verbatim (default N=10), AND a generated summary of the older turns. The summary itself uses the same model with a focused prompt: "summarize key decisions, established facts, open questions, and tool-call outputs from the following conversation in ≤ 500 tokens".
3. **Compressed envelope** — replace older turns with one synthetic `assistant` turn containing `<summary>...</summary>` tags. Workers / tools must continue to see a valid message sequence.
4. **Provenance** — emit audit row (kind: `runtime.context_compressed`) with `{ pre_token_count, post_token_count, summary_length_tokens, turns_compressed }`.
5. **Reversibility** — the original messages stay in `session.messages` (DB); compression only affects the model-input view. So a future inspection / replay can recover the full chain.

This is NOT in scope: model-specific tokenizer integration (assume tokenizer is already plumbed; if not, this brief writes a STUB and a precursor brief is named); cross-turn semantic deduplication; image-compression in vision-enabled models.

## Files you may touch

- `crates/argo-runtime/src/context_compress.rs` (NEW, ≤ 480 LOC)
- `crates/argo-runtime/src/context_compress_tests.rs` (NEW, sibling, ≤ 350 LOC) — 7 tests
- `crates/argo-runtime/src/lib.rs` — narrowly scoped: register module + thread the call site (verify location at claim — likely `agent_loop` or `turn` module)
- `argo-config/src/sections.rs` — `[runtime.context_compress] enabled = true; trigger_pct = 80; keep_last_turns = 10; summary_max_tokens = 500`
- `docs/skycot/RUNBOOK.md` — "Context-window compression" subsection

## Files you must NOT touch

- The DB representation of `session.messages` — compression is a view, not a mutation
- The per-provider tokenizer infrastructure (assumed; if missing, file a precursor brief)

## Acceptance criteria

- [ ] Compression triggers at 80% of model window (configurable)
- [ ] System prompt + last N turns always preserved verbatim
- [ ] Summary generated via same model; ≤ 500 tokens (configurable)
- [ ] Compressed envelope is a valid message sequence (no chain breaks)
- [ ] Audit row emitted with shape `{ pre, post, summary_len, turns_compressed }`
- [ ] DB representation unchanged (only model-input view affected)
- [ ] 7 unit tests with synthetic conversations of varying length
- [ ] `cargo clippy -p argo-runtime --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-context-window-aware-compression`
- PR title: `runtime: context-window-aware compression (mission-context-window-aware-compression)`

## Notes

- The summary call is itself an LLM call — could be expensive. The 80% threshold is the operator-tunable knob; aggressive operators set it to 70% to compress earlier.
- Don't over-summarize. Loose summaries lose the load-bearing detail (specific tool outputs, error messages, decision rationale). Bias the summary prompt toward "preserve facts that future turns may reference".
- Tokenizer per-provider: claude-tokenizer crate for Anthropic, tiktoken-rs for OpenAI. If both aren't already plumbed, file precursor briefs.
