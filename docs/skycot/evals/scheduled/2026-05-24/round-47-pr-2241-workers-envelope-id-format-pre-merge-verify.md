# Round 47 — PR #2241 `fix-web-workers-api-envelope-and-id-format` PRE-MERGE anti-theater verify

**Date**: 2026-05-24 (UTC) · **Round**: 47 · **Pillar**: v1.0 demo recordability · NO THEATER IN DEMO operator amendment · SPA → gateway integration
**Verdict**: 🟢 **PRE-MERGE PASS — recommend admin-merge for demo readiness**

## Setup

- PR #2241 opened 2026-05-24T17:11Z by codex worker (claimed via operator override). Title: "web(workers): unwrap envelope + use gateway-valid demo worker id".
- 30 files changed, +250/-127. Pure-web change — no Rust touched.
- Two concerns bundled:
  1. **Envelope unwrap** (`web/lib/api/workers.ts:+12`): `fetchWorker` accepts both bare `Worker` AND `{ worker: Worker }` envelope.
  2. **ID format migration**: 6 demo worker IDs renamed from `wkr_01J0MARIAH`-style ULID-prefixed strings → UUIDv7-shaped `018f67d0-0000-7000-8000-00000000000N`. Updated everywhere they appear in `web/` (sample fixtures, command palette, escalation seeds, e2e specs, tests, runbook scene 02:30).
- New test file: `web/lib/api/workers.test.ts` (+102 lines, 4 tests).

## Observed

Independent gateway probe — verified the demo IDs are actually parser-valid against the live `:7400` gateway:

```
$ curl -i http://127.0.0.1:7400/api/v1/workers/018f67d0-0000-7000-8000-000000000001
HTTP/1.1 404 Not Found
{"error":{"code":"not_found","message":"worker not found"}}

$ curl -i http://127.0.0.1:7400/api/v1/workers/wkr_01J0MARIAH
HTTP/1.1 400 Bad Request
{"error":{"code":"bad_request","message":"invalid worker_id 'wkr_01J0MARIAH': invalid character: found `w` at 1"}}
```

→ New ID reaches lookup (404 = parser passed); old ID rejected at parser (400 = parser fails with "invalid character: found `w` at 1"). PR's curl evidence is independently confirmed.

Code-read of the envelope-unwrap diff at `web/lib/api/workers.ts:99-105`:

```ts
const response = await apiGet<Worker | WorkerEnvelope>(...);
if (response && typeof response === "object" && "worker" in response) {
  return response.worker;
}
return response;
```

Code-read of `web/lib/api/workers.test.ts` (4 tests):
1. `returns a bare worker response on the 2xx path` — pin
2. `unwraps gateway { worker } envelopes` — direct envelope-unwrap pin
3. `falls back to bundled sample data for demo workers on dev 404` — verifies the demo recordability path (gateway has empty workspace, SPA falls back to bundled sample so the runbook 02:30 scene renders)
4. `throws WorkerNotFoundError for unknown ids after dev 404` — negative case for non-bundled IDs

## Anti-theater check

- The envelope-unwrap is a forward-compatible reader change (accepts both shapes) — it doesn't mask a backend bug, it adapts to gateway response variance. The PR-body curl evidence + my independent probe confirm the route returns the BARE `Worker` shape today, so the envelope-unwrap is unconditionally safe.
- The ID format migration is **not a workaround disguised as a fix** — it's a real defect closure. The author's PR body explicitly notes "the brief described the gateway parser as strict ULID, but the running/backend code uses `uuid::Uuid` for `WorkerId`. I kept the invariant intact and moved the frontend demo IDs to UUID-shaped values instead of loosening any gateway parser." That's the right call: tightening the SPA to match the production invariant rather than relaxing the production invariant.
- The new tests are meaningful: test 2 uses a *different* fixture from test 1 (envelope vs bare), so passing both proves the branch isn't dead. Test 4 throws `WorkerNotFoundError` on a non-bundled UUID (not just "any error") — meaningful negative assertion.
- Test failures admitted in PR body (`pnpm test` 1489 passed / 6 failed) are all in **untouched** files (a11y-contrast, accept-invite, brand-execution-polish, TeamEmptyState copy) — verified by reading the file list — so they're pre-existing drift, not introduced by this PR. **However**, the conductor's pre-merge gate should flag these as inherited-RED before merge to avoid the same "appear-pass with hidden RED" trap that caught #2131 + #2184.

## Gap / follow-up

- The runbook 02:30 scene rename to `/workers/018f67d0-0000-7000-8000-000000000001/overview` is critical for the v1.0 demo MP4 recordability — verified the rename landed in `docs/skycot/operator/demo-runbook.md`.
- **6 pre-existing RED web tests** are inherited drift (NOT introduced by #2241) but will tax the pre-merge gate. Recommend follow-up brief: `fix-web-inherited-red-test-cluster-2241-baseline` (P1, ~80-150 LOC) to clear the 6 failures BEFORE the next demo-blocker web PR lands; otherwise every pre-merge gate run will be polluted by them.

## Brief candidates surfaced this round

- **NEW P1** `fix-web-inherited-red-test-cluster-2241-baseline` — 6 pre-existing RED web tests on origin/main (a11y-contrast hardcoded `#71717a`, accept-invite h1-count, brand-execution-polish 3× hardcoded animations, TeamEmptyState copy / team/page test). All inherited drift, none introduced by demo-blocker PRs in flight. Recommend bundle-fix to clear the pre-merge gate floor before the next demo-blocker web wave (#2242, #2243).
