# fix-gateway-workflows-and-skills-marketplace-endpoints-404

**Owner**: codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-workflows-and-skills-marketplace-endpoints-404`.
**Source**: e2e-ux-audit Agent (onboarding-hire-an-ai-teammate) finding F6 (`docs/skycot/ux-audits/2026-05-25-2309/onboarding-hire-an-ai-teammate.md`).

🟡 **P1 — Multiple top-level gateway endpoints 404 (`/api/v1/skills`, `/api/v1/skills/marketplace`, `/api/v1/workflows`, `/api/v1/workflows/list`, `/api/v1/skill-library/list`). Scenes 4 + 5 cannot run against a live backend.**

## Root cause

Audit-verified curl probes:
- `GET /api/v1/skills` → 404
- `GET /api/v1/skills/marketplace` → 404
- `GET /api/v1/skill-library/list` → 404
- `GET /api/v1/workflows` → 404
- `GET /api/v1/workflows/list` → 404

The SPA `/skills` page falls through to a 6-skill hardcoded seed (see `fix-skills-marketplace-search-seed-competitor-pricing` for the search-result symptom). The SPA `/workflows` page falls through to a SEED_WORKFLOWS fixture. Scene 4 + Scene 5 of the v1.0 demo runbook depend on these surfaces; today the surfaces work only via in-SPA fixtures.

Note: `fix-skills-gateway-route-prefix-and-cors-preflight` addresses the un-prefixed `/skills` (no `/api/v1/`) — that's the path the existing handler responds to. This brief is about adding/exposing the marketplace + listing endpoints under the canonical `/api/v1/*` prefix.

There ARE nested endpoints (`/api/v1/verticals/{vid}/workflows/...`) that exist. The top-level list-style aggregations don't.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F6 [P1] — `/api/v1/skills`, `/api/v1/skills/marketplace`, `/api/v1/workflows`, `/api/v1/workflows/list`, `/api/v1/skill-library/list` all 404

## Fix recipe

1. **Add `GET /api/v1/skills`** (cross-tenant skill catalog). Touch sites:
   - New file `crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs`.
   - Wire into the router (`crates/argo-cli/src/handlers/gateway/router.rs` or equivalent).
   - Returns `{ "skills": [{ id, name, description, layer, risk, version, tags, ... }], "total_count": N }`. Per-tenant if the caller has installed skills; cross-tenant catalog if requesting `marketplace=true`.

2. **Add `GET /api/v1/skills/marketplace`** (the un-installed catalog). Same handler can serve both with a query param, or split into a sibling route.

3. **Add `GET /api/v1/workflows`** (cross-vertical workflow list for the caller's tenant). Touch:
   - New file `crates/argo-cli/src/handlers/gateway/routes_workflows_list_top_get.rs`.
   - Returns `{ "workflows": [{ id, name, vertical, step_count, version, ... }], "total_count": N }`.
   - This is the top-level aggregation; the existing per-vertical route stays.

4. **`/api/v1/workflows/list` and `/api/v1/skill-library/list`** are likely the older URL shapes the SPA tries as fallbacks. Either:
   - (a) Add them as aliases of the canonical routes, OR
   - (b) Update the SPA fallback paths to use the canonical names only.

5. **CORS**: ensure each new route also responds to OPTIONS preflight (per `mission-gateway-cors-preflight`).

6. **Tenant scope**: every list-style endpoint MUST filter by the caller's tenant. Add a cross-tenant negative test for each.

7. **Tests**:
   - `crates/argo-cli/tests/gateway_skills_list_smoke.rs`
   - `crates/argo-cli/tests/gateway_workflows_list_smoke.rs`
   - Each: seed N rows for tenant A, seed M rows for tenant B; assert tenant A's list returns exactly N items, none of B's.

## Acceptance criteria

- [ ] `curl GET /api/v1/skills` returns 200 with a skills envelope (not 404).
- [ ] `curl GET /api/v1/skills/marketplace` returns 200 with the un-installed catalog.
- [ ] `curl GET /api/v1/workflows` returns 200 with the workflows envelope.
- [ ] `curl GET /api/v1/workflows/list` returns 200 OR (operator's call) the SPA stops calling it.
- [ ] OPTIONS preflight returns 204 with CORS headers on all new routes.
- [ ] Cross-tenant negative tests pass.
- [ ] SPA `/skills` and `/workflows` pages render live data (verify by toggling the existing `dataSource` chip from `mock-offline` to `gateway`).

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_workflows_list_top_get.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/router.rs` (wire-up)
- `crates/argo-cli/tests/gateway_skills_list_smoke.rs` (NEW)
- `crates/argo-cli/tests/gateway_workflows_list_smoke.rs` (NEW)
- `web/lib/api/skills.ts` and `web/lib/api/workflows.ts` (if SPA fallback paths need updating)

## Out of scope

- Skill installation (separate brief `fix-skills-detail-install-button-and-flow`)
- Search-result seed for "competitor pricing scrape" (separate brief `fix-skills-marketplace-search-seed-competitor-pricing`)
- The MCP+CU workflow seed (separate brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Sub-workflow picker static fixture (separate brief `fix-workflow-builder-subworkflow-picker-uses-static-fixture`)

## Effort

~200-400 LOC across 2 new handlers + 2 tests + router wire-up. ~4-6 hours.
