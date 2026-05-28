# fix-spa-genesis-catch-overwrites-sse-error-event-message

**Severity**: 🟡 P1 — MVP demo polish (paired with `fix-spa-genesis-fetch-fails-while-curl-succeeds`)

**Authored against**: `origin/main` SHA `e62dd99fa` + working-tree CORS-on-SSE fix on 2026-05-28T04:30Z.

**Discovered during**: 2026-05-28 e2e walkthrough after the CORS-on-SSE fix for `fix-spa-genesis-fetch-fails-while-curl-succeeds` unblocked the SPA → gateway path.

## Symptom

After the SSE response carries CORS headers (the C fix), the SPA's `/genesis` page connects to the gateway successfully:
- OPTIONS preflight 204
- POST 200 + SSE stream with `Content-Type: text/event-stream` + `Access-Control-Allow-Origin: <origin>`
- 4 `event: progress` chunks arrive
- (for retries-on-existing-persona) `event: error` arrives mid-stream with the gateway's idempotency message
- stream closes with `0\r\n\r\n` terminator + TCP FIN

But the SPA renders the error card with the text "network error" — not the gateway's actual error message ("write vertical: output dir ... not empty; refusing to overwrite an existing vertical").

## Why

`web/app/(app)/genesis/page.tsx` line 122-138 has two error-handling paths for the same fetch:

```ts
streamGenesis(req, {
  onProgress: (ev) => setActivePhase(ev.phase),
  onVerticalReady: (ev) => { setVertical(ev); setStatus("ready"); },
  onError: (ev) => {
    setErrorEvent(ev);  // ← SSE-event error message (the gateway's actual text)
    setStatus("error");
  },
}).catch((err) => {
  if (controller.signal.aborted) return;
  setErrorEvent({
    phase: activePhase ?? "drafting_persona",
    message: err instanceof Error ? err.message : "Couldn't reach the Genesis stream.",
    recoverable: true,
  });
  setStatus("error");
});
```

The flow is:
1. `onError` callback fires when the SSE stream emits an `event: error` payload → sets `errorEvent` to the gateway's actual message
2. The stream continues reading until close (the gateway does `write_terminator` + `out.shutdown()`)
3. AFTER the stream closes, the .catch fires anyway with err.message == "network error" (a chrome TypeError text — likely caused by the abrupt `out.shutdown()` half-close)
4. .catch **overwrites** the errorEvent with the generic "network error" text

Net: the camera sees a generic "network error" instead of the specific gateway-reason ("output dir is not empty").

## Two fix paths

### Path A — SPA-side: don't overwrite a populated errorEvent (1-line fix)

```ts
}).catch((err) => {
  if (controller.signal.aborted) return;
  // Don't overwrite an error event already set by the SSE `onError`
  // callback (e.g. the gateway's specific reason). Only fill in the
  // generic "couldn't reach" message when no event-derived error has
  // been recorded.
  setErrorEvent((prev) =>
    prev ?? {
      phase: activePhase ?? "drafting_persona",
      message: err instanceof Error ? err.message : "Couldn't reach the Genesis stream.",
      recoverable: true,
    }
  );
  setStatus("error");
});
```

This is a minimal change. The functional update on `setErrorEvent` only sets the generic message when the SSE-event-derived message is absent (true network failures).

### Path B — gateway-side: don't half-close the connection abruptly after `event: error`

Look at `crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs::write_error_event`:

```rust
async fn write_error_event(
    out: &mut TcpStream,
    phase: &str,
    message: &str,
    recoverable: bool,
) -> tokio::io::Result<()> {
    write_event(out, "error", &payload).await?;
    out.flush().await?;
    write_terminator(out).await?;
    out.shutdown().await   // ← drop this; let the connection close naturally
}
```

Drop `out.shutdown()`. The chunked-encoding terminator + the function returning (which lets the caller close the socket) is enough.

**Recommendation**: ship Path A — it's a smaller, more localised fix that also helps any future SSE quirks. Path B is the deeper correctness fix and can ship later as a follow-up.

## Acceptance criteria

For Path A:

- [ ] On a Genesis retry where `verticals/.genesis/<tenant>/<persona>/` already exists, the SPA renders the gateway's actual message ("write vertical: output dir 'verticals/.genesis/op-…/' is not empty; refusing to overwrite an existing vertical"), NOT "network error"
- [ ] On a true network failure (gateway off / unreachable), the SPA renders the fallback "Couldn't reach the Genesis stream." (or chrome's TypeError text)
- [ ] On a successful Genesis flow (clean tenant + persona dir), the SPA renders the PreviewCard with the synthesized worker, NO error card
- [ ] No regression on existing genesis page tests

## May-touch

- `web/app/(app)/genesis/page.tsx` (the catch handler at line 128-138)
- new vitest covering the "onError fires + then .catch fires → onError's message preserved" case

## References

- Companion brief: `fix-spa-genesis-fetch-fails-while-curl-succeeds` — closed by the gateway-side CORS-on-SSE fix in commit (this commit)
- Companion P1: `fix-genesis-idempotency-output-dir-not-empty` — addresses the root cause of repeated `event: error` payloads, which makes this brief moot once landed
- Walkthrough source: `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` Scene 3
