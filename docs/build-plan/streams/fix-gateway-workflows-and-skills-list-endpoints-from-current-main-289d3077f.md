# fix-gateway-workflows-and-skills-list-endpoints-from-current-main-289d3077f - re-author top-level gateway list endpoints

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `a84871a22` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-workflows-and-skills-list-endpoints-from-current-main-289d3077f`.

## Why this is P1

PR #2323 stayed open through six evaluator/conductor rounds even after source-PASS reviews. The live gateway on port 7400 still returns `404` for both top-level endpoints:

- `GET /api/v1/skills`
- `GET /api/v1/workflows`

These routes back the skills and workflow browsing surfaces used in the v1.0 demo path. The stalled branch should not be rebased again; re-author the small endpoint slice from current main.

## Scope

Implement the endpoint intent directly on current main:

1. Mount `GET /api/v1/skills` as a caller-tenant installed-skill list endpoint.
2. Mount `GET /api/v1/skills/marketplace` and `GET /api/v1/skill-library/list` as published marketplace aliases if the handler naturally supports them.
3. Mount `GET /api/v1/workflows` as a caller-tenant top-level workflow-definition list endpoint.
4. Mount `GET /api/v1/workflows/list` as an alias if the handler naturally supports it.
5. Keep tenant filtering load-bearing:
   - installed skills are filtered by caller tenant
   - workflows are filtered by caller tenant
   - marketplace rows may intentionally span tenants only for published records
6. Add focused smoke tests that seed tenant A and tenant B rows and prove tenant B cannot read tenant A private rows.

You may read PR #2323 as reference material, but do not modify, rebase, or force-push its branch.

## Current router shape

Current main uses `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` as the domain-resource GET coordinator. Do not add module declarations to `crates/argo-cli/src/handlers/gateway/mod.rs`: it is already at the 500-LOC ceiling. Keep this slice self-contained by declaring the two new handler modules from `routes_dispatch_get.rs` with narrow `#[path = "..."]` module declarations, or by another equivalent pattern that keeps every touched file under 500 LOC.

`crates/argo-cli/src/handlers/gateway/routes_workflows.rs` currently owns the `/api/v1/verticals/{vertical_id}/workflows/...` subtree only; it does not match `/api/v1/workflows`. `crates/argo-cli/src/handlers/gateway/routes_skills.rs` currently owns non-API `/skills...` routes and does not match `/api/v1/skills`.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
- `crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs` - new
- `crates/argo-cli/src/handlers/gateway/routes_workflows_list_top_get.rs` - new
- `crates/argo-cli/tests/gateway_skills_list_smoke.rs` - new
- `crates/argo-cli/tests/gateway_workflows_list_smoke.rs` - new

## Files you must NOT touch

- PR #2323's branch.
- Any web files.
- Any schema migrations.
- `crates/argo-cli/src/handlers/gateway/mod.rs` unless a conductor explicitly widens the boundary after splitting or shrinking it below the LOC ceiling.

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/api/v1/skills` returns HTTP 200 with a JSON list envelope after running the new branch's gateway binary.
- [ ] `curl http://127.0.0.1:7400/api/v1/workflows` returns HTTP 200 with a JSON list envelope after running the new branch's gateway binary.
- [ ] Cross-tenant smoke tests prove tenant-private skills and workflows do not leak across tenants.
- [ ] Existing `/api/v1/verticals/{vertical_id}/workflows/...` routes still dispatch through `routes_workflows`.
- [ ] All touched files stay under the 500 LOC ceiling.

## Verification

- `cargo test -p argo-cli --test gateway_skills_list_smoke --test gateway_workflows_list_smoke`
- `cargo clippy -p argo-cli --all-targets -- -D warnings`
- `bash scripts/codex-pre-push.sh` for Codex workers before pushing.

## Branch + PR

- Branch: `codex-fix-gateway-workflows-and-skills-list-endpoints-from-current-main-289d3077f` or `claude-...`.
- PR title: `fix(gateway): mount top-level skills and workflows list endpoints (fix-gateway-workflows-and-skills-list-endpoints-from-current-main-289d3077f)`.
