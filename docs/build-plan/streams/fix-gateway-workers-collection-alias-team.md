# fix-gateway-workers-collection-alias-team

**Owner**: codex.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-workers-collection-alias-team`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md` F04 [P1].

🟡 **P2 — Gateway `/api/v1/workers` (collection) returns 404; only `/api/v1/team` returns the list. But `/api/v1/workers/{id}` (singleton) works fine. API surface inconsistency that will trip future code readers, external API consumers, and connector authors.**

## Root cause

The list-workers endpoint lives at `/api/v1/team` (handler at `crates/argo-cli/src/handlers/gateway/routes_team_list.rs`). The resource is also addressable at `/api/v1/workers/{id}`. The HTTP semantics of REST suggest `GET /api/v1/workers` should return the collection — but no handler is registered for that path. The router returns 404.

This is a v0-era naming artifact: `/team` was named when "team" was the product term for the worker roster. The product has since adopted "Workers" as the canonical noun in the SPA nav (per memory `project_workers_page_design_locked`), but the API endpoint was never aliased or renamed.

Curl evidence:
```
curl :7400/api/v1/workers       → 404 {"error":"not found"}
curl :7400/api/v1/workers/<id>  → 200 worker object
curl :7400/api/v1/team          → 200 workers list
```

Not a demo blocker — the SPA uses `useTeam()` which hits `/api/v1/team`. P2 because consistency matters for API consumers + future-self.

## Findings this brief closes

- whole-site-mvp-claims-coverage F04 [P1, downgraded to P2] — `/api/v1/workers` collection 404, only `/api/v1/team` returns list

## Fix recipe

Option A (recommended — add alias, deprecate old):
1. **Add a `/api/v1/workers` GET handler** in `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs` (NEW) that calls the same underlying service function as `/api/v1/team`.
2. **Mark `/api/v1/team` as deprecated** in OpenAPI spec / response header (`Deprecation: true; sunset="2027-01-01"`). Keep it serving for back-compat with v0 consumers.
3. **Update SPA client** `web/lib/api/team.ts` to point at `/api/v1/workers`; rename the function `fetchTeam` → `fetchWorkers` (preserve `useTeam()` hook name temporarily for diff minimality, or rename to `useWorkers`).
4. **Smoke test**: both endpoints return the same payload shape.

Option B (gateway-only alias, no SPA migration):
1. Add a route alias at the gateway level — `GET /api/v1/workers` → same handler as `GET /api/v1/team`. No SPA changes.
2. Simpler, lower-risk; preserves both forever.

**Recommendation: Option A**. The product calls them "Workers"; the API should too. Deprecate `/team` cleanly.

## Acceptance criteria

- [ ] `curl :7400/api/v1/workers` returns 200 with the same payload shape as `/api/v1/team`.
- [ ] `curl :7400/api/v1/team` still returns 200 (back-compat); response includes a `Deprecation: true` header.
- [ ] SPA `web/lib/api/team.ts` switched to call `/api/v1/workers` (function and hook may stay `useTeam()` for now; brief follow-up to rename).
- [ ] OpenAPI spec at `crates/argo-cli/openapi.yaml` (or equivalent) lists both endpoints with `/team` marked deprecated.
- [ ] Integration test in `crates/argo-cli/tests/` covers parity between the two endpoints.
- [ ] No SPA regression: `/team` page (which calls `useTeam()`) renders correctly.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs` (NEW — the alias handler)
- `crates/argo-cli/src/handlers/gateway/routes_team_list.rs` (add Deprecation header)
- `crates/argo-cli/src/handlers/gateway/router.rs` (or equivalent — wire the new route)
- `web/lib/api/team.ts` (point at `/api/v1/workers`)
- `crates/argo-cli/tests/gateway_workers_list_alias.rs` (NEW)
- `crates/argo-cli/openapi.yaml` (if present)

## Out of scope

- Renaming the SPA function/hook `useTeam()` → `useWorkers()` (deferred to a follow-up; keeps this brief diff small)
- Renaming `/team` UI page (already in transition per `fix-workers-team-route-ambiguity-and-breadcrumb-drift`)
- Per-worker subroutes (`/duties`, `/schedules`) — separate (`fix-worker-detail-responsibilities-schedules-tab-end-to-end`)

## Effort

~100-150 LOC across 1 new handler + router wire + SPA pointer + 1 test. ~2-3 hours.
