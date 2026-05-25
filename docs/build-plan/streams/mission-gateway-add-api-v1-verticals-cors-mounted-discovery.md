# Stream mission-gateway-add-api-v1-verticals-cors-mounted-discovery — mount `/api/v1/verticals` discovery endpoint

**Owner**: codex (gateway / handler work; ~150-250 LOC including helper + tests).
**Depends on**: none. Sibling brief `fix-web-verticals-route-cors-preflight-symmetric-fix` covers the web caller side; either order works.
**Authored against**: origin/main `234ffd9f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-add-api-v1-verticals-cors-mounted-discovery`.

🟡 **P2.** Source: round-49 evaluator finding `gateway-vertical-discovery-sibling-gap-confirm` + r51 carry-forward (5th-consecutive round). The gateway already serves nested vertical-scoped routes (`/api/v1/verticals/{vid}/workflows/...`, `/api/v1/verticals/{vid}/workflows/{name}/test_runs`, etc.) but has no discovery endpoint returning the list of available verticals — yet `web/lib/api/workflows.ts:7` documents `GET /verticals → discovery` as the contract.

## Scope

Add `GET /api/v1/verticals` returning the list of available verticals for the caller's tenant. Shape:

```json
{ "verticals": [ { "id": "pr_checker", "name": "PR Checker", "workflow_count": 7 }, ... ] }
```

Tenant-bound: the list reflects verticals visible to the authenticated tenant (per the standard `Authorization` / session-cookie tenant-resolution path that every other gateway route uses). Empty workspace returns `{ "verticals": [] }`, never 404.

Implementation pattern matches existing list-style routes — for example see `crates/argo-cli/src/handlers/gateway/routes_workflows_list_get.rs` (per-vertical listing) for the response-envelope, tenant-scope, and error conventions.

Mount the route in the gateway's route table (search for `routes_workflows_list_get::list_response` to find the existing mount block; add a sibling mount for the new handler).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [gateway route-table mount file]

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_verticals_list_get.rs` (NEW, ~80-120 LOC) — handler + response shape + tenant resolution + count-per-vertical query.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — add `pub mod routes_verticals_list_get;` and re-export the handler if the surrounding modules do.
- The gateway route-table dispatch file (find via `rg "routes_workflows_list_get::list_response" crates/argo-cli/src/handlers/gateway/`) — mount `GET /api/v1/verticals → routes_verticals_list_get::list_response`.
- `crates/argo-cli/src/handlers/gateway/routes_verticals_list_get_tests.rs` (NEW) or inline `#[cfg(test)]` — 3+ tests covering: (a) empty workspace returns `{verticals: []}`; (b) populated workspace returns expected list with workflow counts; (c) cross-tenant isolation — tenant A's verticals don't leak to tenant B.

## Tests

- 3 unit tests as above (~80 LOC total).
- 1 integration test under `crates/argo-cli/tests/` that boots the gateway with the SQLCipher harness, seeds 2 tenants × 2 verticals, hits the endpoint with each tenant's session, and asserts the lists don't cross.

## Definition of done

- New handler file exists, under 500 LOC.
- Route mounted; `cargo test -p argo-cli --test gateway_verticals_list` (or whatever the integration file is named) green.
- Cross-tenant negative test included and passes.
- PR body includes `curl -i http://127.0.0.1:7400/api/v1/verticals` against a populated gateway showing the response envelope.
- Sibling brief `fix-web-verticals-route-cors-preflight-symmetric-fix` will also be unblocked once this lands; cite the cross-link in the PR body.

Notes: tenant-binding is non-negotiable — model the WHERE clause after `crates/argo-store/src/skill_query.rs` recent tenant-pin fixes (#2260). No SELECT without a `tenant_id = ?` predicate.
