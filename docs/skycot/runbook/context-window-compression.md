# Context-window compression

Long-running sessions can blow past a model's context-window cap
(200K tokens for Claude Sonnet 4.x, 128K for `gpt-4o`, etc.). The
runtime reacts before the provider rejects the call: when the
running token-count crosses a configurable trigger (default 80% of
`model.context_window_tokens`), older turns are summarised into a
single synthetic `assistant` message wrapped in
`<summary>…</summary>`, and the rest of the history (system prompt
+ last N user turns, default N=10) is kept verbatim.

The DB representation of `session.messages` is left untouched —
compression is a *view*, applied to the next turn's model input. A
later replay or `argo audit query session-replay` recovers the
original chain.

## Operator knobs (`argo.toml`)

```toml
[runtime.context_compress]
enabled = true            # master enable
trigger_pct = 80          # fire at 80% of model context window
keep_last_turns = 10      # trailing user turns kept verbatim
summary_max_tokens = 500  # cap on the generated summary
```

Aggressive operators set `trigger_pct = 70` to compress earlier
(useful when the summariser itself is on a small model and you want
budget headroom for it). Setting `enabled = false` short-circuits
the path entirely — the runtime falls back to the previous
truncate-or-fail behaviour and is responsible for that downside.

## Audit trail

Each compression pass writes one `runtime.context_compressed` audit
row with payload:

```json
{
  "pre_token_count": 184320,
  "post_token_count": 47210,
  "summary_length_tokens": 463,
  "turns_compressed": 42
}
```

`argo audit query --kind runtime.context_compressed` answers "how
often is this session paying the summarisation cost?" and feeds the
operator's decision on whether to drop `trigger_pct` further or
shorten `keep_last_turns`.

## What this is NOT

- **Not lossless.** The summary is a model-generated paraphrase;
  exact wording from collapsed turns is gone from the model-input
  view. The DB still has the originals if you need them.
- **Not a replacement for shorter sessions.** Compression buys
  headroom; it does not make the model retain everything.
- **Not multimodal-aware.** Image / audio content blocks are not
  compressed in v1 — only text turns participate in the summary.

## Fallback: adaptive truncation

Compression is the preferred path, but it leans on an LLM call and so
can itself fail (rate-limit, refusal, network). It also cannot help
when a *single* turn is too big to fit — a 200K-token tool dump
cannot be summarised below 200K tokens without losing the load-bearing
output. For both cases, `argo_runtime::context_truncate` provides a
deterministic fallback:

- Drops older user/assistant turns FIFO; never touches the system
  prompt, the last user turn, the last assistant turn, or
  `Tool`-role messages within the last 3 user-turn boundaries.
- As a last resort, replaces a single oversized turn (default
  >50% of the window) with `[...truncated XXX tokens of tool
  output...]` so the provider transport accepts the call.
- Writes one `runtime.context_truncated` audit row per pass with
  payload `{ pre_token_count, post_token_count, turns_dropped,
  intra_turn_truncations }`.
- Hard-fails with `ArgoError::ContextWindowExhausted` when even the
  minimum (system prompt + last user turn) cannot fit — the caller
  must handle, never silently accept.

`argo audit query --kind runtime.context_truncated` answers "how
often did the deterministic fallback kick in?" — a non-zero rate
relative to `runtime.context_compressed` is the operator's signal to
investigate compression failures.
