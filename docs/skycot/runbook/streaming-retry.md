# Streaming Retry

The provider runtime can wrap streaming LLM calls in
`argo_provider::streaming_retry`. If a stream fails after emitting
text, the wrapper buffers the assistant text, waits with exponential
backoff, and opens a fresh stream with that text pre-filled as the
last assistant message. The caller sees one continuous stream rather
than a full restart.

Configure it in `argo.toml`:

```toml
[runtime.streaming_retry]
enabled = true
max_attempts = 3
backoff_initial_ms = 1000
```

`enabled = false` makes streaming calls pass through unchanged.
`max_attempts` counts retries after the initial attempt, so `3`
allows four total attempts. `backoff_initial_ms` doubles for each
retry and is capped internally so a bad value cannot stall a stream
indefinitely.

Each retry emits a structured warning with
`kind = "runtime.streaming_retry"`, the attempt number, delay,
source error, and buffered byte count. Tail it through the standard
tracing to log-shipping pipeline.

After the retry budget is exhausted, the wrapper returns
`ArgoError::StreamingRetryExhausted { partial_content }`. CLI and API
surfaces should render the partial text with an interruption marker
instead of discarding it.

V1 buffers text deltas only. Tool-call and reasoning deltas are not
stitched across attempts; the model is expected to re-emit them on
the continuation attempt.
