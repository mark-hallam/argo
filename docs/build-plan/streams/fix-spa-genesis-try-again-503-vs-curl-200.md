# fix-spa-genesis-try-again-503-vs-curl-200

**Priority**: P0 — v1.0 demo MP4 Scene 3 (Genesis "Try again") blocker
**Source**: 2026-05-28 interactive re-walk under Genesis-MVP scope
**Authored**: 2026-05-28 by claude conductor (interactive session)

## The bug

On the live SPA at `/genesis`, when the operator submits a role description
that hits the idempotency refusal (because `verticals/.genesis/<tenant>/<vertical_id>/`
already exists from a prior attempt), the SPA correctly surfaces the SSE
`error` event ("output dir … is not empty; refusing to overwrite an
existing vertical") with a `Try again` button (per #2622 catch-handler fix).

But clicking `Try again`, which per #2630 is supposed to send `{ overwrite: true }`
in the next POST, **returns HTTP 503 to the browser** instead of streaming
a successful vertical_ready event. The SPA's UI falls back to a generic
"network error" banner.

## Reproduction

1. Start gateway at HEAD (4e4e773b2 or later — contains #2630 + #2622).
2. Start SPA at HEAD with `NEXT_PUBLIC_GATEWAY_URL=http://localhost:7400 pnpm dev`.
3. Navigate to `http://localhost:3000/genesis`.
4. Submit any role description ≥ 15 chars.
5. If `verticals/.genesis/<tenant>/<vertical_id>/` does not pre-exist, the
   first attempt succeeds and there's nothing to test. Pre-populate it
   (or use the second attempt) so the first attempt hits the idempotency
   error.
6. After the SSE error event, click `Try again`.
7. Network tab shows POST `http://localhost:7400/api/v1/genesis` → 503.
8. UI shows "network error" banner (generic, not the SSE error event).

## What works (the diagnostic asymmetry)

Curl with the **exact same headers + body** the SPA sends — including
`Content-Type`, `Accept: text/event-stream`, `X-Argo-Tenant-Id: <tenant>`,
`Origin: http://localhost:3000`, plus optional `Cookie`, `Referer`,
`Sec-Fetch-*`, `User-Agent` — returns **HTTP 200 + full SSE stream**
(all 5 phases + `event: vertical_ready`). Confirmed during the 2026-05-28
walkthrough multiple times back-to-back, milliseconds before the SPA
attempt that 503'd.

So the gateway code path for `overwrite: true` works. The bug is whatever
makes the SPA's browser fetch hit a different code path inside the
gateway.

## Hypotheses (in order of likelihood)

1. **HTTP request-line mismatch.** `is_genesis_streaming_request()` checks
   `req.starts_with("POST /api/v1/genesis ")` (trailing space) or
   `"POST /api/v1/genesis?"`. If the browser fetch produces a slightly
   different request line — e.g. with a query string the SPA didn't
   explicitly add, or a fragment, or some Next.js dev-server proxying
   that rewrites the path — it could fall through to a different
   dispatcher arm and return a 503 from a non-genesis path (perhaps
   `shutdown.rs`'s shutting_down_response if the rate limiter or some
   middleware misfires).

2. **OPTIONS preflight side-effect.** Browser sends OPTIONS before POST.
   The network tab shows the OPTIONS returns 204 cleanly, but maybe the
   gateway has a 1-shot consumed-state mid-request that the second OPTIONS
   triggers something.

3. **Connection reuse / keep-alive.** Browser may reuse the TCP connection
   from a prior POST. The gateway's raw-socket SSE handler may not handle
   a second request on the same connection well, leading to a 503 from
   the next dispatch arm.

4. **Cookies / session middleware.** Browser sends session cookies (from
   prior magic-link login or signup), and a session-validating middleware
   returns 503 when the session is unknown/expired. Curl with explicit
   `Cookie: skycot-session=abc123` returns 200 though, so this is less
   likely.

## What to investigate

- Add a trace log at the top of `serve_genesis_stream` printing the
  full request line + first 5 headers. Repro from SPA. Compare to curl.
- Check `boot_router.rs:148` to see if there's an early return for some
  path the SPA hits but curl doesn't.
- Strace / tcpdump the gateway socket for 30s while reproducing the bug
  to see the literal HTTP/1.1 request bytes the SPA sends vs curl.
- Inspect whether the browser is upgrading to HTTP/2 (`h2c` Upgrade
  header) on localhost — argo's gateway is HTTP/1.1 only and may
  mis-handle an Upgrade attempt.

## Acceptance

- SPA's `Try again` button posts with `overwrite: true` and receives
  HTTP 200 + full SSE stream (Drafting persona → Compiling charter →
  Generating skill bundle → Validating eval seeds → Ready →
  `event: vertical_ready`).
- A new integration test in
  `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_tests.rs`
  pins the second-attempt-with-overwrite path returning 200 through
  the same code path the SPA exercises (whatever the actual bug is).
- E2E manual: `/genesis` submit → Try again → vertical_ready in the
  UI, no "network error" banner.

## Out of scope (deferred to follow-up briefs)

- Refactor of the streaming-vs-non-streaming dispatch in `boot_router.rs`
  (current shape is correct, just under-instrumented).
- Adding HTTP/2 support to the gateway.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs`
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_tests.rs`
- `crates/argo-cli/src/handlers/gateway/boot_router.rs` (if dispatch is the cause)
- `web/lib/api/genesis.ts` (if SPA fetch shape is the cause — unlikely)

## Notes

- Verified gateway uptime ~17min, build_commit `4e4e773`, schema_version 84.
- Provider key (`ANTHROPIC_API_KEY`) is set and curl-direct path works.
- This is NOT a regression of #2630 or #2622 — both individual fixes
  are sound. The bug is in the interaction between SPA fetch + gateway
  dispatch for the SECOND POST in the same browser session.
