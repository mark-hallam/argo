# Round 48 — PR #2247 `fix-web-workflows-route-mismatch` PRE-MERGE anti-theater verify

**Date**: 2026-05-24 (UTC) / 2026-05-25 (Melbourne)
**Round**: 48
**Pillar / Claim**: v1.0 demo-blocker — SPA `/workflows` catalog needs to render against the live gateway. PR is part of the demo-blocker fix wave (#2241 / #2242 / #2243 / #2247).
**Verdict**: 🟢 **PRE-MERGE PASS for the brief's scoped fix. 🟡 surfaces a parallel sibling gap in `web/lib/api/verticals.ts` that the brief is correctly scoped not to touch — 1 follow-up brief recommended.**

## Setup

PR head OID `686a5f1337` (claude worker, opened 2026-05-24 18:14 UTC). 141 additions / 82 deletions across 2 files:

- `web/lib/api/workflows.ts` (+40 / -30) — production
- `web/lib/api/workflows.test.ts` (+101 / -52) — test

Verification:

1. `git fetch origin pull/2247/head:pr-2247-head` — pin PR head.
2. Read `workflows.ts` and `workflows.test.ts` end-to-end on PR head; compare against `origin/main`.
3. `git grep '"/verticals"' pr-2247-head -- web/` — verify the dead endpoint is no longer reached from the fixed file AND check for sibling callers.
4. `git grep '"/verticals"' origin/main -- web/` — confirm sibling callers are pre-existing, not introduced.

## Observed

### Diagnosis correctness

The PR body lays out the failure mode crisply: `/verticals` lives at the root path on the gateway, but the CORS preflight handler (`crates/argo-cli/src/handlers/gateway/cors.rs:144`) matches only `/api/v1/*`. So a signed-in browser's `apiGet("/verticals")` triggers a non-simple preflight (`Authorization: Bearer` header) and the OPTIONS request 404s. The actual GET never fires; `fetch()` throws `TypeError("Failed to fetch")` and the catalog silently degrades to mock-offline seed.

Independent diagnostic curl pair in the PR body proves it: `OPTIONS /verticals → 404` vs `OPTIONS /api/v1/verticals/sales-sdr/workflows → 204 No Content + CORS headers`. Reproducible.

### Production fix (`workflows.ts`)

Replaces the dead `apiGet<VerticalsEnvelope>("/verticals")` round-trip with a local-seed-derived vertical list:

```ts
function discoverVerticalIds(): string[] {
  const seen = new Set<string>();
  for (const wf of SAMPLE_WORKFLOW_SUMMARIES) {
    if (wf.vertical) seen.add(wf.vertical);
  }
  return Array.from(seen);
}
```

Then fans out per-vertical against the CORS-good `/api/v1/verticals/<id>/workflows` route, with per-bucket recoverable-error swallow (401/403/404/5xx/network → empty bucket in non-strict env) and a fall-through to `SAMPLE_WORKFLOW_SUMMARIES` if every bucket comes back empty. Strict prod (`NODE_ENV=production` AND `NEXT_PUBLIC_ENV=production`) still propagates 5xx — load-bearing because that's what keeps a broken prod deploy visible instead of masked by seed.

### Test surface — load-bearing regression pin

`workflows.test.ts` ships **8 vitest cases**, of which **3 are load-bearing**:

1. `"never hits the legacy /verticals discovery endpoint"` (`:240-258`) — inspects every URL `fetchMock` was called with and asserts NONE ends with `/verticals` or matches `/verticals(\\?|$)/`. This is the canonical regression pin: if the dead endpoint ever sneaks back into `fetchAllWorkflows`, this test goes red.
2. `"treats per-vertical 401 / 403 as empty so the catalog still renders"` (`:159-181`) — proves auth-failure-on-one-vertical doesn't blank the whole catalog (pre-fix behaviour).
3. `"propagates per-vertical 5xx in strict production"` (`:215-228`) — proves the strict-prod escape hatch is real (not swallowed). Negative-side guard against silently masking a broken prod deploy with seed data.

The remaining 5 tests cover happy-path aggregation, all-empty fallback, 404 fallback, mock-bypass, and per-vertical-mixed-status swallow.

## Anti-theater check

| Failure mode | How this PR refuses it |
|---|---|
| Test passes vacuously | The `"never hits /verticals"` test enumerates `fetchMock.mock.calls` and rejects on substring match — would fail immediately if any `/verticals` URL fires. |
| Strict-prod escape masks real outage | Dedicated test pins 503 propagation as `ApiError` with status 503; not just "throws something". |
| 401 / 403 lumped with 404 | Separate test cases for 401, 404, 5xx — proves each is recoverable, not coincidentally same shape. |
| Sample-vs-gateway shape drift | Test items use `definition_toml` shape, exercises `parseStepsFromToml` decoder. Returns 3 steps + `approval-required` risk — non-trivial fan-out → decode round-trip. |
| Brief scope creep | Diff is strictly inside `web/lib/api/workflows.{ts,test.ts}` (per may-touch list). No gateway / SPA-page edits. |

## Sibling gap — pre-existing, recommend follow-up brief

🟡 **Independent grep of PR branch finds the same dead-CORS-preflight pattern in `web/lib/api/verticals.ts:202`:**

```
pr-2247-head:web/lib/api/verticals.ts:202:    const envelope = await apiGet<VerticalsEnvelope>("/verticals");
```

This is the `fetchVerticals()` function — used by `web/lib/hooks/use-verticals.ts:21` (React Query `queryFn`) which in turn drives the `web/app/(app)/verticals/page.tsx` index page. So:

- Signed-in browser → `/verticals` SPA page → `fetchVerticals` → `apiGet("/verticals")` → CORS preflight 404 → `Failed to fetch` → seed fallback → page shows mock data.
- Same identical failure mode as the workflows catalog, on a sibling SPA route.

Confirmed pre-existing — `origin/main:web/lib/api/verticals.ts:202` already contained this. **NOT a regression introduced by #2247.** The brief is web-only and correctly scoped to workflows; the sibling is a separate brief opportunity.

## Gap / follow-up brief candidates

**(a) `fix-web-verticals-route-cors-preflight-symmetric-fix` (P1, ~50-100 LOC web)** — apply the SAMPLE-derived seed-fallback recipe from PR #2247 to `fetchVerticals()` in `web/lib/api/verticals.ts:197-215`. Either (i) skip the `/verticals` round-trip entirely and derive the vertical list from `SAMPLE_VERTICALS` (mirrors #2247's `discoverVerticalIds` approach), or (ii) reach the same endpoint via the CORS-good `/api/v1/verticals` if the gateway exposes one (per PR body the "gateway-side reconcile" is explicitly out of scope here). Symmetric vitest pin: `never hits the legacy /verticals discovery endpoint` test mirrors the workflows.test.ts:240 case. Without this, the SPA `/verticals` index page silently degrades to seed under signed-in browser. **Demo-recordability check**: if `/verticals` is in the recorded v1.0 demo MP4 surface, this is demo-blocker shape; if not, P1 / web-only / can ship post-demo. Recommend conductor confirm with operator before promote to P0.

**(b) `mission-gateway-add-api-v1-verticals-cors-mounted-discovery` (P2, ~30-60 LOC Rust)** — alternative root-cause fix on the gateway side: mount `/api/v1/verticals` so it inherits the CORS preflight handler in `cors.rs:144`. PR body flags this as out-of-scope and explicitly defers it to a follow-up brief; this is that follow-up shape. P2 not P1 because the web-side fix (a) is the demo-blocking path; the gateway-side reconcile is the long-term-correct fix but doesn't block the demo.

## Verdict

🟢 **PRE-MERGE PASS for the scoped brief.** PR #2247 is anti-theater-honest, the regression pin is load-bearing, and the brief is correctly bounded. Strongly recommend admin-merge after the inherited-red-test cluster (already flagged via round-47 brief `fix-web-inherited-red-test-cluster-2241-baseline`) lands or is ledger-noted as out-of-scope for this PR.

Process note: this is the **26th anti-theater-honest substrate contract verification this week.** The sibling-gap finding is the kind of pre-existing drift the evaluator → conductor loop catches early — a small follow-up brief now is much cheaper than discovering it via mid-demo `/verticals` page failure.
