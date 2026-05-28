# fix-spa-genesis-fetch-fails-while-curl-succeeds

**Severity**: 🔴 P0 — MVP demo blocker (Genesis is operator-promoted MVP-must-have per [genesis-is-mvp-must-have-2026-05-28 memory](../../skycot/agents/heartbeats/conductor.json))

**Authored against**: `origin/main` SHA `718fb9f6d…` on 2026-05-28T03:50Z. Validate before claim: `bash scripts/check-brief-freshness.sh fix-spa-genesis-fetch-fails-while-curl-succeeds`.

**Discovered during**: 2026-05-28 e2e walkthrough with operator. Notes at `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` (Scene 3 section).

## Symptom

Click "Generate worker" on `/genesis` → SPA shows "Failed to fetch" error card. Chrome devtools network panel reports `POST http://localhost:7400/api/v1/genesis → 503 Service Unavailable` (also `POST http://127.0.0.1:7400/api/v1/genesis → 503` when `NEXT_PUBLIC_GATEWAY_URL` is the 127.0.0.1 form).

## Why this needs a focused brief (not vibes)

Backend `/api/v1/genesis` **definitely works**. Direct curl from terminal — with identical headers, identical body, same gateway process — returns a clean 200 OK + full SSE stream:

```
event: progress data: {"phase":"drafting_persona","percent":10,...}
event: progress data: {"phase":"compiling_charter","percent":35,"message":"Compiled archetype: lead_researcher"}
event: progress data: {"phase":"generating_skill_bundle","percent":65,...}
event: progress data: {"phase":"validating_eval_seeds","percent":85,...}
event: progress data: {"phase":"ready","percent":100,...}
event: vertical_ready data: {"manifest":{"actions_declared":5,"archetype":"lead_researcher",...},"prompts":[...],"skills":[],"eval_seed_count":10,"judge_verdict":"pass","tenant_id":"op-44bf79bb...","vertical_id":"lead-researcher-accounts","vertical_dir_path":"verticals/.genesis/op-.../lead-researcher-accounts"}
```

So this is not a Genesis-pipeline bug, not a provider-key bug (env-var `ANTHROPIC_API_KEY` is set on the gateway process — confirmed via `ps eww -p <gateway-pid>`), not a tenant-resolution bug (curl works with both ad-hoc `op-<rand>` slugs and real provisioned tenant UUIDs).

## What's been ruled out

- ✅ Backend works (curl 200 + SSE)
- ✅ Provider key env present (`ANTHROPIC_API_KEY=sk-ant-api03-XXXX...` visible on `argo gateway` process env)
- ✅ Tenant slug doesn't matter (curl works with both `op-<rand>` and real UUIDs)
- ✅ CORS OPTIONS preflight succeeds (204 with `Access-Control-Allow-Origin: http://localhost:3000` + all required `Access-Control-Allow-Headers`)
- ✅ Genesis idempotency edge case (dir-already-exists) is a SEPARATE issue — see `fix-genesis-idempotency-output-dir-not-empty.md` (companion brief authored alongside this one)
- ✅ Field-name match — SPA at `web/lib/api/genesis.ts:339` sends `role_description` per gateway expectation
- ✅ My initial "field-name mismatch" diagnosis was wrong (per `feedback_verify_before_fix_check_existing_prs.md`)

## Diagnostic candidates to investigate

In likely-cause order:

### 1. Chrome Private Network Access (PNA) enforcement

Both `localhost:3000 → localhost:7400` and `localhost:3000 → 127.0.0.1:7400` are public-to-private cross-origin POSTs under chrome's PNA spec. The OPTIONS preflight response does NOT include `Access-Control-Allow-Private-Network: true`.

**To test**: add `Access-Control-Allow-Private-Network: true` to the gateway's CORS preflight response when the request includes `Access-Control-Request-Private-Network: true`. Retry SPA fetch.

Code site: `crates/argo-cli/src/handlers/gateway/cors.rs` — CORS handler emits Allow-Origin/Methods/Headers/Credentials/Max-Age but not Allow-Private-Network.

### 2. Connection-reset masquerading as 503

Chrome devtools sometimes reports the HTTP status from a partially-received response when the connection drops mid-stream. The SPA's fetch() promise rejects with `TypeError: Failed to fetch`, which is a NETWORK error not an HTTP error. If the gateway closes the connection between headers and body (e.g. by exiting the handler abnormally), chrome may attach "503" to the panel entry while the JS fetch sees a TypeError.

**To test**: run `tcpdump -i lo0 -w /tmp/genesis-spa.pcap port 7400` while clicking Generate. Inspect the pcap with wireshark for the actual bytes the gateway sent. If the gateway closed without finishing the response, this is the cause.

### 3. Browser-side SSE-POST restriction

Browsers historically have quirks with POST + `Accept: text/event-stream` (the standard `EventSource` API uses GET only). The SPA uses `fetch()` with manual stream parsing in `web/lib/api/genesis.ts:354+`, not `EventSource`. Some chrome versions may apply different CORS / preflight rules to fetch() responses with `Content-Type: text/event-stream`.

**To test**: try changing the SPA's `Accept` header to `application/x-ndjson` or `*/*` and see if the fetch succeeds. If so, switch the response wire format or use Server-Sent Events properly.

### 4. Gateway logging gap

Gateway log `~/.argo/logs/gateway-20260528T123742.log` shows nothing about the SPA's 503 attempts — only unrelated `audit_watchdog` noise. Either the 503 path bypasses tracing, or the request never reaches the handler.

**To test**: temporarily add `tracing::warn!` at the top of `serve_genesis_stream` (before any check) and confirm log shows arrival when SPA fetches. If no log, request didn't reach the handler — confirms the 503 is from a middleware (cors / maintenance / something else) or from chrome itself.

## Acceptance criteria

- [ ] Click "Generate worker" on `/genesis` from a fresh chrome session against the running SPA on `:3000` and live gateway on `:7400`
- [ ] SSE stream renders in the ProgressStrip — all 5 phases visible (drafting_persona / compiling_charter / generating_skill_bundle / validating_eval_seeds / ready)
- [ ] PreviewCard renders with the synthesized worker (name, archetype, charter excerpt, skill bundle list)
- [ ] Clicking the page's "Hire" / commit button calls `POST /api/v1/workers` and navigates to `/workers/<new-worker-id>` with the synthesized worker visible on the profile page
- [ ] Re-run the same description on the same tenant slug works idempotently (does NOT 503 / "output dir not empty") — gated on companion brief `fix-genesis-idempotency-output-dir-not-empty`

## May-touch

- `crates/argo-cli/src/handlers/gateway/cors.rs` — likely add Allow-Private-Network response header
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs` — possibly add tracing / fix connection handling
- `web/lib/api/genesis.ts` — possibly adjust Accept header or fetch options
- `web/app/(app)/genesis/page.tsx` — possibly surface gateway error.message instead of generic "Failed to fetch"

## Must-not-touch

- The Genesis pipeline crates (`argo-genesis`) — the backend works, don't refactor
- The SPA's existing SSE consumer pipeline shape — keep the streamGenesis/hireFromGenesis split

## References

- Operator promoted Genesis to MVP-must-have: 2026-05-28T03:30Z
- Backend Genesis curl test (success): `2026-05-28-e2e-dry-run-with-operator.md` Scene 3
- Prior shipped Genesis PRs: #876, #1408, #1599, #1623, #2084, #2109, #2119, #2361, #2400 (first-hire invoke), #2420 (provider-key 503 path), #2520, #2553. All MERGED. The plumbing exists.
