# Stream fix-web-workflows-route-mismatch — reconcile /workflows frontend endpoints with gateway

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `f3d1c66a6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-web-workflows-route-mismatch`.
🔴 **P0 DEMO-BLOCKER.** `/workflows` returns 500 in the browser. Diagnosed: frontend `web/lib/api/workflows.ts` calls `apiGet<VerticalsEnvelope>("/verticals")` and `apiGet<WorkflowListEnvelope>(path)` — but the gateway's `/api/v1/workflows` returns `{"error":"not found"}`, and the frontend's `/verticals` call also likely fails. The frontend↔gateway routing for workflows is misaligned.

## Scope

Audit the workflows route on both sides and reconcile. Likely either:
- Frontend uses outdated endpoint paths that the gateway has since renamed (e.g. `/verticals` → `/api/v1/workflows/verticals` or similar)
- Gateway endpoints were never wired (the brief that would have added them was deferred)
- Frontend expects a different envelope shape than the gateway returns

Implementer:
1. Read `web/lib/api/workflows.ts` to find every endpoint path the frontend hits
2. For each path, curl the gateway at `http://localhost:7400{path}` (with `ARGO_DEMO_MODE=1` if needed for principal mint)
3. Reconcile: either update the frontend path, or add the missing gateway route, or unwrap the envelope shape
4. Verify `/workflows` and `/workflows/sales/qualify_lead/build` both render in the browser

The demo voiceover at 4:00-6:30 walks through `/workflows` + the React Flow canvas at `/workflows/sales/qualify_lead/build` — both need to work for the demo.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/lib/api/workflows.ts` — primary surface; update endpoint paths and/or envelope unwrap
- `web/app/(app)/workflows/page.tsx`, `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — only if the page-level data fetch shape needs to change (likely not — keep changes in the API layer)
- `web/lib/sample-workflows.ts`, `web/lib/sample-workflow-runs.ts` — if envelope shapes change
- If a gateway endpoint genuinely doesn't exist and needs adding, FLAG and STOP — mark BLOCKED with reason "missing gateway endpoint: /api/v1/<path>" and the conductor will queue a separate `mission-gateway-workflows-list-endpoint` brief.

## Files you must NOT touch

- Any `crates/argo-*/**` file — gateway changes are a separate brief. This brief is web-side only.
- Other API client files unless they share helpers being refactored
- Any web/app/(app)/ page outside the workflows tree

## Acceptance criteria

- [ ] `curl http://localhost:3000/workflows` returns 200 + rendered HTML (no 500, no Next.js error page)
- [ ] `/workflows` renders the catalog list in the browser (template tiles or empty state, not a crash)
- [ ] `/workflows/sales/qualify_lead/build` renders the React Flow canvas
- [ ] `pnpm typecheck` clean
- [ ] `pnpm test` — no regressions
- [ ] PR body includes a 1-paragraph "what was misaligned" summary with the curl command that diagnosed it
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-fix-web-workflows-route-mismatch`
- PR title: `web(workflows): reconcile API endpoints with gateway (fix-web-workflows-route-mismatch)`

## Notes for the implementer

- Diagnosed during the 2026-05-24 demo walkthrough. Sibling: `fix-web-mock-bypass-env-var-rename` (which makes the mock fallback work for everything) + commit `1d1085aab` (which fixed the analogous envelope drift on `/team`).
- If you find the gateway endpoint truly missing, do not try to add it in this brief — that's a Rust crate change, different scope. Mark BLOCKED, queue a `mission-gateway-workflows-list-endpoint` brief.
- The React Flow canvas at `/workflows/sales/qualify_lead/build` is the centerpiece of the demo voiceover ("Drag and drop the nodes — pan, zoom, mini-map. Just like n8n or UIpath. But also — describe a change in plain English."). If the canvas doesn't render even after the data fix, that's a separate UI issue worth noting.
