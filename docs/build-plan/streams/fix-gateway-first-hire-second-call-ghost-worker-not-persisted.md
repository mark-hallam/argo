# fix-gateway-first-hire-second-call-ghost-worker-not-persisted

**Owner**: codex.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-first-hire-second-call-ghost-worker-not-persisted`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F2 [P0].

🔴 **P0 — `POST /api/v1/onboarding/first-hire` returns `201 {worker_id}` for the second hire per tenant, but the worker is never persisted. `GET /api/v1/workers/{id}/overview` returns 404 forever and the worker doesn't appear in `/api/v1/team`. SPA's `verifyWorkerReadable` retries 3 × 500 ms then surfaces "Hiring succeeded but your teammate isn't visible yet." Any non-demo signup that hires a second teammate hits a permanent ghost.**

## Root cause

Live curl evidence against `http://127.0.0.1:7400`:

```
$ # FIRST hire, vertical=marketing-intern
$ POST /api/v1/onboarding/first-hire {"description":"Sara...","vertical_id":"marketing-intern"}
   201 {"name":"Sara","worker_id":"019e6674-ecb0-...","vertical_id":"marketing-intern"}
$ GET /api/v1/workers/019e6674-ecb0-.../overview
   404 {"error":{"code":"not_found"}}
$ GET /api/v1/team
   200 { total_count:1, workers:[{ id:"019e5dbb-c03d-...", name:"Maya" }]}   # the FIRST-EVER hire
```

The endpoint returns a freshly minted UUID that never lands in storage. Tested against `marketing-intern` + `sales-sdr` verticals with two different descriptions — same outcome: only the very first hire ever per tenant persists. Subsequent hires return fresh worker_ids that never reach the `workers` table.

Suspect a `one_per_tenant` guard in `crates/argo-cli/src/handlers/gateway/onboarding/first_hire.rs` (or equivalent) that short-circuits with a fabricated UUID after the first successful insert. Either (a) returns a new ID without writing, or (b) writes to a tenant-scoped row that the read-side ignores, or (c) the "name extraction → conflict on (tenant_id, name) UNIQUE" path silently swallows the conflict and returns the existing row's ID under a new transaction.

SPA-side this compounds with F1 (demo-mode short-circuit) — when demo is OFF, the user clicks "Hire them", waits ~1.5s, then sees `FIRST_HIRE_VERIFY_FAILED_MESSAGE` ("Hiring succeeded but your teammate isn't visible yet. Try refreshing or contact support."). There is no support route to contact.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F2 [P0] — Gateway `first-hire` second call per tenant returns 201 but never persists; UI shows "isn't visible yet"

## Fix recipe

1. **Source-walk the gateway handler.** Locate `POST /api/v1/onboarding/first-hire` route in `crates/argo-cli/src/handlers/gateway/onboarding/`. Read the persist path end-to-end and find where the second call diverges.
2. **Identify the failure mode** — is it (a) silent-skip (no INSERT executes), (b) INSERT-then-rollback, or (c) INSERT-into-wrong-scope (`tenant_id` not bound, lands in a tenant the read-side can't see)?
3. **Fix the path so every hire persists** AND returns the canonical worker_id. If the desired semantics is "idempotent — second hire returns the existing worker's ID", switch to `409 Conflict {worker_id: <existing>}` so the SPA can route to the canonical row instead of fetching a fresh ghost UUID.
4. **Add a regression test** in `crates/argo-cli/tests/` that exercises the handler twice per tenant with two different descriptions, asserts both worker_ids exist in `/api/v1/workers/{id}/overview` AND in `/api/v1/team`.
5. **SPA-side defence**: in `web/lib/onboarding/use-first-hire.ts:verifyWorkerReadable`, if the verify-probe 404s, fall back to refetching `/api/v1/team` and finding the worker by name match before surfacing the error toast. Reduces blast radius if the gateway regresses again.

## Acceptance criteria

- [ ] `curl -X POST` twice against `/api/v1/onboarding/first-hire` with two different descriptions and a single `Bearer` token: both worker_ids resolvable via `/api/v1/workers/{id}/overview` (200) AND both rows appear in `/api/v1/team`.
- [ ] Cargo test asserts the above invariant.
- [ ] SPA verify-probe path: if gateway 404s on overview, fall back to `/api/v1/team` lookup-by-name; only surface `FIRST_HIRE_VERIFY_FAILED_MESSAGE` if neither path resolves.
- [ ] Investor demo path: real-mode (demo OFF) second hire succeeds without the "isn't visible yet" message.

## May-touch

- `crates/argo-cli/src/handlers/gateway/onboarding/first_hire.rs` (suspect)
- `crates/argo-cli/src/handlers/gateway/onboarding/` (peer modules — vertical install, persist)
- `crates/argo-cli/tests/gateway_first_hire_test.rs` (NEW)
- `web/lib/onboarding/use-first-hire.ts` (verify-probe fallback)
- `web/lib/api/team.ts` (lookup-by-name helper if not present)

## Out of scope

- Demo-mode short-circuit (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`)
- Name extraction quality (covered by `fix-onboarding-first-hire-name-parser-prefers-named-pattern`)
- Workers collection endpoint 404 (covered by `fix-gateway-workers-collection-route-alias-404-add-canonical`)

## Effort

M — ~150-250 LOC across handler + test + SPA fallback. ~3-4 hours. P0 because second-hire failure is permanent for any real-tenant user; the demo MP4 is recordable on first-hire-only happy path but any customer-trial use is a hard block.
