# sprint-ws3-4 — de-mock the remaining mock-leaking production API modules

**Authored against:** origin/main `56a98b258` on 2026-06-05
**Validate before claim:** `bash scripts/check-brief-freshness.sh sprint-ws3-4-demock-remaining-mock-leaking-api-modules`
**Owner:** claude · **Workstream:** WS3 (de-mock) · **Lane:** frontend TS (no cargo)

## Why
A public / paying signup must NEVER see fixture data on a camera-visible surface.
The SPA serves a `sample-*` / 404-fallback mock when the gateway auth bypass is on.
The correct guard is `isGatewayAuthBypassed() && !isDemoLive()` — i.e. serve mock ONLY
in dev-bypass AND NOT in demo-live; in demo-live, hit the REAL endpoint.

**2026-06-05 audit (`web/lib/api/*.ts`):** only **33 of ~67 production modules** carry the
`isDemoLive` gate. The ungated production modules can still serve their mock in demo-live.

## Scope — fix the ungated production modules that have a real endpoint + a mock fallback
Priority (camera-visible for a paying signup), in order:
1. `usage.ts` (billing/usage meter — `GET /api/v1/tenant/usage`)
2. `workspace-members.ts`
3. `connectors.ts`
4. `workflow-execute.ts`, `workflow-metadata.ts`, `workflow-rollback.ts`, `workflows-save-publish.ts`
5. `inbound-channels.ts`
6. `sso.ts`

## Per-module rule (apply, do not blanket-edit)
For each module above:
- `rg 'isDemoLive|isGatewayAuthBypassed' web/lib/api/<module>.ts` first (grep-before-write).
- Confirm a real **dispatched** gateway endpoint exists for the path it calls (check
  `crates/argo-cli/src/handlers/gateway/routes_catalog.rs` + the dispatch arm). **Wire-or-defer:**
  - Endpoint EXISTS → add `&& !isDemoLive()` so demo-live calls the live endpoint; mock fires only under dev-bypass-and-not-demo-live.
  - Endpoint does NOT exist → **do NOT hide the surface** (that was WS3-1's premise-false mistake). Leave it, and list it in the PR body as a real backend gap for a follow-up brief.
- Intentional mock/seed (`retrain-mocks.ts`, `demo-roster-store.ts`, `skills-seed.ts`) → skip + note.

## Explicitly EXCLUDE (already correct — do not re-touch)
- `orchestration / workers / worker-trust / workflow-runs / workplace-knowledge` — verified by WS3-3 (#2700).
- `retros / duties / retrain` — WS3-1 confirmed real endpoints, bypass-gated, deep-link/inline by design.
- `voice / use-realtime-voice / use-voice-turn / websocket` — real by design (live voice shipped).
- `genesis / pitch / gtm` — real LLM, truth-bounded.
- All `*.test.ts` files (tests, no gate needed).

## Verify
- `npx tsc --noEmit` clean + `vitest run` green on touched files.
- Per fixed module: a pinning vitest asserting the LIVE fetch fires under demo-live, and the mock fires ONLY under `isGatewayAuthBypassed() && !isDemoLive()`.
- NO THEATER: do not fabricate data; if an endpoint is missing, say so in the PR body.

## PR
Title: `sprint(ws3-4) — de-mock — gate remaining mock-leaking API modules behind !isDemoLive()`
Keep ≤ the LOC ceiling; if it grows, split by surface (billing/usage vs workflow vs sso).
