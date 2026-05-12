# Mission load-bearing wire — Streaming-aware retry (don't lose mid-stream content)

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 8 #249. When a streaming LLM call fails mid-stream (network blip, rate-limit, transient 5xx), the retry today restarts from scratch and discards everything received so far. For long completions, that's expensive in tokens + wall-clock + UX.

## Scope

NEW `argo-provider::streaming_retry` module providing a `retry_streaming<F>(req, attempt_fn)` wrapper:

1. **Capture mid-stream content** in the wrapper — every `chunk` received before failure is buffered.
2. **On transient failure** (network error / 5xx / 429 with Retry-After): retry the call with the original prompt + a continuation hint: `{ assistant_prefix: <buffered content so far> }`. The provider call appends to the prefix instead of starting over.
3. **Attempt budget** — max 3 retries with exponential backoff (1s, 2s, 4s).
4. **Stream stitching** — emit the buffered chunks downstream first, then the new stream's chunks. Consumers see one continuous stream.
5. **Provider-specific continuation** — Anthropic supports `assistant_prefix` via the `assistant` role pre-fill pattern; OpenAI supports continuation via assistant message; document both per-provider in the brief.
6. **Hard-fail** after attempts exhausted: return `ArgoError::StreamingRetryExhausted` with the partial content as a recoverable hint.

This is NOT in scope: idempotency keys for non-streaming calls (separate brief); cross-provider stitching (each provider retries against itself); content-deduplication (assume the model picks up cleanly from the prefix).

## Files you may touch

- `crates/argo-provider/src/streaming_retry.rs` (NEW, ≤ 400 LOC)
- `crates/argo-provider/src/streaming_retry_tests.rs` (NEW, sibling, ≤ 400 LOC) — 7 tests with mockito
- `crates/argo-provider/src/lib.rs` — narrowly scoped: register module + plumb into the streaming-call path (verify location)
- `argo-types::ArgoError` — add `StreamingRetryExhausted { partial_content }` variant
- `argo-config/src/sections.rs` — `[runtime.streaming_retry] enabled = true; max_attempts = 3; backoff_initial_ms = 1000`
- `docs/skycot/RUNBOOK.md` — "Streaming retry" subsection

## Files you must NOT touch

- Per-provider transport implementations (the wrapper is generic over Transport trait)
- The non-streaming retry path

## Acceptance criteria

- [ ] Mid-stream failure → retry with `assistant_prefix` containing buffered content
- [ ] Stream-stitching: consumer sees buffered chunks first, then continuation chunks
- [ ] Max 3 retries with exp backoff (1s/2s/4s)
- [ ] `StreamingRetryExhausted` typed error with `partial_content` payload
- [ ] Audit row per retry (kind: `runtime.streaming_retry`) — visible in dashboard
- [ ] 7 unit tests covering: clean-retry / 3-retries-exhausted / non-retryable-error / etc.
- [ ] `cargo clippy -p argo-provider --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-streaming-aware-retry`
- PR title: `provider: streaming-aware retry with continuation (mission-streaming-aware-retry)`

## Notes

- Anthropic's continuation pattern: pre-fill the assistant role with the buffered content. The model continues from where it left off (often within a sentence — but that's OK; the stitched output reads naturally).
- The 3-retry budget is operator-tunable. Don't make it infinite — pathologically failing endpoints would burn the token budget on the same prompt forever.
- Test with a mock streaming server that disconnects mid-stream. `mockito` + `tokio_test` is the pattern.
