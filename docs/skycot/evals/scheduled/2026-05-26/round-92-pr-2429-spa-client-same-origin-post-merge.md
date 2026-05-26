# Round 92 — PR #2429 spa-client same-origin POST-MERGE verification

- **Date / UTC**: 2026-05-26T19:15Z
- **Round**: 92 (prior 91 PRE-MERGE PASS'd this PR at 18:25Z; closing the loop)
- **Pillar / Claim**: Claim 6 (Multi-tenant by construction — SPA cannot consume gateway without same-origin proxy in browser) + demo-MP4 ship-gate (T0119 F01/F02, scene-1-2 F1, scene-4-5 F6)
- **Verdict**: 🟢 **PASS** — anti-theater dispositive, 92nd verification this week.

## Setup
- `git fetch origin main` → HEAD `ecfb68191`; merge commit for #2429 is `51ab5c6dd3f950c8a76a6dc6ccc5da0f9fe5a2fe` (2026-05-26T18:33Z UTC / 04:33 AEST 05-27).
- `git show 51ab5c6dd --stat`: 52 LOC across `web/lib/api/client.ts` (+6) and `web/lib/api/client.test.ts` (+46).
- Read `web/lib/api/client.ts` lines 80-88 against current `origin/main`.
- Searched the wider client tree for any remaining hard-coded cross-origin gateway URLs.

## Observed
Production code at `web/lib/api/client.ts:80-88` (current `origin/main`):

```typescript
// Browser-side fetches go through Next.js's same-origin rewrite at
// `/api/v1/*` (see `web/next.config.ts`) so the browser never crosses
// origins and CORS never enters the picture. SSR and route-handler
// contexts run inside the Next.js process and must use the absolute
// gateway URL because the Next process is the requester, not a browser.
function gatewayBase(): string {
  if (typeof window !== "undefined") return "";
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}
```

- `typeof window !== "undefined"` returns the browser path correctly because at SSR module evaluation `window` is `undefined` (Node global); on the client `window` is defined.
- The return value `""` composes with `${gatewayBase()}${path}` at line 185 to produce paths like `/api/v1/audit?range=today` — same-origin from the browser's perspective.
- `web/next.config.ts:30-32` rewrite (unchanged) proxies `/api/v1/:path*` to the gateway, so the SPA hits Next on `:3000` and Next reverse-proxies to the gateway at `:7400`. CORS preflight never fires.

## Anti-theater check

1. **Production code matches the test contract.** R91 verified `client.test.ts` 13/13 with real `globalThis.fetch` mock + `vi.stubGlobal("window", undefined)` + negative regex (`not.toMatch(/^https?:/)`). Both code-and-test landed; both are reachable from `main`.
2. **Cold cargo not relevant** (TypeScript-only change) — instead checked that the test file imports the real `apiGet`/`apiPost` exports and exercises the path through `request<T>()`, not a parallel implementation.
3. **No leftover absolute URLs in client paths**: `grep -rn 'NEXT_PUBLIC_GATEWAY_URL' web/lib/api/` returns only `client.ts:87` (the SSR-fallback that's gated on the `typeof window` branch). No `http://127.0.0.1:7400` literals leaking through other client paths.
4. **Downstream tests caught by the contract change** (3 of them: `audit.test.ts`, `orchestration.test.ts`, `workers.test.ts`) were inline-fixed in commit `56f4ea0b4` by relaxing `.toBe('http://gw.test/...')` to `.toMatch(/\/api\/v1\/...$/)` — verified in this round's scenario 3 below. The new regex still pins **path + query string**, so the assertion is meaningful, not a green-stamp.
5. **R91's PRE-MERGE PASS forecast held.** R91 said "anti-theater dispositive on two independent axes per test"; POST-MERGE the production diff is byte-for-byte the head commit R91 verified (`c9f9718e7` matches `51ab5c6dd^{tree}` for these two files).

## Demo impact
- T0119 F01 (P0 `/audit` permanent loading) — root cause was browser cross-origin fetch → CORS rejection → silent loading state. Same-origin path fix unblocks.
- T0119 F02 (P0 cross-origin CORS cascade) — closed by the same one-line branch.
- scene-1-2 F1 (P0 `/workers` mock-offline chip) — root cause was the same CORS failure; chip surfaced the failure but didn't mention CORS.
- scene-4-5 F6 (P1 `/skills` SSR flash) — closed by the SSR branch returning the absolute URL so the server-side render hits the gateway directly without flashing through a same-origin path that needs hydration.

## Gap / follow-up
None for #2429. Process drift finding (50-cycle ghost-tick zombie-claims pre-merge) already escalated in R91 — recommend the conductor surface the `ops-supervisor-block-runaway-zombie-claims-on-brief` brief candidate to the operator. (Evaluator does not author briefs.)

## Recommendation
Close R91 PRE-MERGE forecast as **confirmed POST-MERGE**. Mark #2429 as a clean win in the conductor's next tick summary. No further follow-up required from this scenario.
