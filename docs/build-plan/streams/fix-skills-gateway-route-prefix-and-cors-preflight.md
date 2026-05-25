# fix-skills-gateway-route-prefix-and-cors-preflight

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-gateway-route-prefix-and-cors-preflight`.
**Source**: e2e-ux-audit Agent 3 finding F6 (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🟡 **P1 — SPA's skill API call is talking to a 404 endpoint; CORS preflight fails. Cross-tenant isolation can't even be tested.**

## Root cause

The SPA hook `useSkills()` at `web/lib/api/skills.ts:142` calls `apiGet("/skills")`, which resolves to `http://127.0.0.1:7400/skills` (NOT the prefixed `/api/v1/skills`). Agent 3 curl-verified:

```
$ curl -X OPTIONS http://127.0.0.1:7400/skills          → 404
$ curl http://127.0.0.1:7400/skills                      → 200 (empty envelope)
$ curl -X OPTIONS http://127.0.0.1:7400/api/v1/skills    → 204
```

The browser blocks the call at CORS preflight (Cross-Origin Resource Sharing — browser rule that controls when a webpage on one origin can talk to a server on another origin). The SPA hook errors, the page falls through to the 6-card seed list. The page APPEARS fine but is structurally talking to nothing — `/skills` looks live to the user but is actually fallback-mocked.

This is **also** the upstream cause of why cross-tenant isolation can't be tested on `/skills`: a fresh install via the broken API never propagates. Coupled with `fix-skills-detail-install-button-and-flow` — without the route fix, the install brief's cross-tenant test cannot pass.

## Findings this brief closes

- Agent 3 F6 [P1] — Gateway `OPTIONS /skills` returns 404; the SPA browser-fetch fails CORS preflight; SPA silently falls through to 6-card seed

## Fix recipe

Three options; pick (a) for narrowest scope:

**(a) Re-point SPA at the prefixed route (preferred)**:
1. In `web/lib/api/skills.ts:142`, change `apiGet("/skills")` to `apiGet("/api/v1/skills")`.
2. Sweep other SPA endpoints for un-prefixed calls: `grep -nE 'apiGet|apiPost.*"/[^a]' web/lib/api/`. Each un-prefixed call is a candidate failure of the same kind. Fix every match.
3. Add a CI lint: `scripts/check-api-prefix.sh` that scans `web/lib/api/*.ts` for un-prefixed routes, fails if any exist.

**(b) Add OPTIONS preflight handler for the un-prefixed `/skills` route**:
1. In the gateway router, register an OPTIONS handler that returns 204 with the standard CORS headers for any un-prefixed route the SPA happens to call. This works but masks the underlying inconsistency.

**(c) Migrate all gateway routes to `/api/v1/...` and 410 the un-prefixed ones**: too broad for this brief.

Recommendation: ship (a). It surfaces every other instance of the same defect and prevents new ones via the lint.

## Acceptance criteria

- [ ] `web/lib/api/skills.ts` (and any sibling files found in the sweep) call `apiGet("/api/v1/skills")` not `apiGet("/skills")`.
- [ ] DevTools Network panel on `/skills` shows a successful 200 on `/api/v1/skills` (no fail row).
- [ ] `useSkills()` returns the gateway response when the gateway has skills seeded, NOT the 6-card seed fallback.
- [ ] CI lint added: any `web/lib/api/*.ts` call missing `/api/v1/` prefix fails CI.
- [ ] curl `-X OPTIONS http://127.0.0.1:7400/api/v1/skills -H "Origin: http://localhost:3000"` returns 204 with CORS headers.

## May-touch

- `web/lib/api/skills.ts:142` + any other un-prefixed callers
- `scripts/check-api-prefix.sh` (NEW lint)
- `.github/workflows/*` (only if a workflow needs to wire the lint — currently disabled per `.github/workflows/DISABLED_WORKFLOWS_POLICY.md`)
- `crates/argo-cli/src/handlers/gateway/cors.rs` (only if a CORS-side fix is preferred)

## Out of scope

- Skill install button + flow (separate brief: `fix-skills-detail-install-button-and-flow`)
- Seeding "competitor pricing scrape" skills (separate brief: `fix-skills-marketplace-search-seed-competitor-pricing`)
- The broader source-pill MOCK/LIVE problem on `/workflows` (separate brief: `fix-workflows-page-mock-pill-eyebrow-on-camera`)

## Effort

~30-80 LOC + sweep + lint. ~45-90 min.
