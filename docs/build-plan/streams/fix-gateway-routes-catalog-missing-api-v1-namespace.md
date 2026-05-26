# fix-gateway-routes-catalog-missing-api-v1-namespace

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-routes-catalog-missing-api-v1-namespace`.
**Source**: ux-e2e-audit-T2309MEL `onboarding-hire-an-ai-teammate.md` F8 [P1] + `whole-site-mvp-claims-coverage.md` F28 [P2].

🟠 **P1 — `curl http://127.0.0.1:7400/routes | jq` returns 143 routes, none under `/api/v1/*`. But `POST /api/v1/workers`, `GET /api/v1/audit`, `GET /api/v1/approvals`, and the rest of the v1 namespace all actually respond when hit directly. The route catalog silently omits the entire v1 namespace, mis-leading any new agent debugging the SPA→gateway wiring into thinking those endpoints don't exist. Also: `/api/v1/openapi.json`, `/api/v1/tenants`, `/api/v1/connectors` all return 404 — the catalog should either list them as 404'd-on-purpose or implement them.**

## Root cause

The gateway's `/routes` discovery endpoint at (likely) `crates/argo-cli/src/handlers/gateway/routes.rs` builds its catalog from a static registry. The dispatchers in `crates/argo-cli/src/handlers/gateway/routes_dispatch_*.rs` (e.g. `routes_dispatch_get_api_groups.rs`, `routes_dispatch_post_paths.rs`) DO register `/api/v1/*` paths and respond to them — but the discovery catalog doesn't read from the dispatchers; it reads from a separate registry that the v1 routes were never added to.

Evidence:
```bash
$ curl http://127.0.0.1:7400/routes | jq '. | length'
143

$ curl http://127.0.0.1:7400/routes | jq '[.[] | .path] | map(select(startswith("/api")))'
[]

$ curl -X POST http://127.0.0.1:7400/api/v1/workers -d '{}'
# returns 400 (validation error, route exists)
```

Additional 404s the audit catalogued:
```
curl :7400/api/v1/workers           → 404 not found (per scene-1-and-2 F3 — confirmed live)
curl :7400/api/v1/tenants           → 404 not found
curl :7400/api/v1/connectors        → 404 not found
curl :7400/api/v1/openapi.json      → 404 not found
```

(Note: `/api/v1/workers` 404 is being separately addressed by `fix-gateway-workers-collection-alias-team`. The catalog brief is the meta-fix that prevents a future similar discovery gap.)

This is a discoverability landmine, not a journey-blocker. The brief's curl checklists across the wider audit batch routinely fail because new agents trust `/routes` to be complete, then conclude the endpoint they need doesn't exist when in fact it just isn't catalogued.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F8 [P1] — `/api/v1/workers` route missing from `/routes` catalog
- whole-site-mvp-claims-coverage F28 [P2] — Multiple `/api/v1/*` endpoints return 404 (workers/tenants/connectors/openapi.json) with no catalog visibility

## Fix recipe

1. **Find the catalog builder.** `rg 'pub fn routes_catalog\|pub fn list_routes\|"/routes"' crates/argo-cli/src/handlers/gateway/` to locate the response builder. Likely `routes.rs` or `routes_catalog.rs`.

2. **Audit completeness.** Compare the catalog response against the dispatcher registrations:
   ```bash
   rg "fn dispatch.*GET\|fn dispatch.*POST" crates/argo-cli/src/handlers/gateway/
   ```
   Build a script `scripts/check-routes-catalog-complete.sh` that:
   - Hits `/routes` and extracts the `path` array
   - Greps all `routes_dispatch_*.rs` files for the path patterns they register
   - Asserts each dispatched path appears in the catalog (or is explicitly excluded with a comment)

3. **Surface `/api/v1/*` in the catalog.** Add the v1 namespace routes to whatever registry the catalog reads. The minimum set (per audit + observed dispatchers):
   - `GET /api/v1/team`, `GET /api/v1/workers`, `GET /api/v1/workers/{id}`, `GET /api/v1/workers/{id}/overview`
   - `POST /api/v1/workers`, `PATCH /api/v1/workers/{id}`
   - `GET /api/v1/audit`, `GET /api/v1/audit/{id}`, `POST /api/v1/audit/{id}/replay`
   - `GET /api/v1/approvals`, `POST /api/v1/approvals/{id}/decide`
   - `GET /api/v1/workflows`, `GET /api/v1/verticals/{v}/workflows/{n}`
   - `GET /api/v1/skills`, `GET /api/v1/skills/marketplace`, `POST /api/v1/skills` (install)
   - `GET /api/v1/cohorts`
   - `POST /api/v1/genesis` (currently 503; covered by `fix-genesis-orphan-discoverability-and-provider-key-503`)

4. **Decide on `/api/v1/openapi.json`.** Either:
   - Implement an OpenAPI 3.x document generated from the route catalog (M-effort, follow-up brief), OR
   - Add to the catalog as `path: "/api/v1/openapi.json", status: "not_implemented"` to signal intent

5. **Add a CI gate.** `scripts/check-routes-catalog-complete.sh` runs in `.github/workflows/gateway-routes-complete.yml` (or the local pre-merge gate script) on every PR touching `crates/argo-cli/src/handlers/gateway/`.

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/routes | jq '[.[] | .path] | map(select(startswith("/api/v1")))'` returns >= 15 paths
- [ ] Every route the SPA's `web/lib/api/*.ts` fetches appears in the catalog
- [ ] `scripts/check-routes-catalog-complete.sh` exits 0 against current main
- [ ] CI workflow added (or local pre-merge-gate step) that runs the script
- [ ] `cargo build --release -p argo-cli` green
- [ ] `cargo test -p argo-cli routes_catalog` (existing or new) green

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes.rs` (or `routes_catalog.rs` — verify) (~40 LOC additions)
- `crates/argo-cli/src/handlers/gateway/tests/routes_catalog.rs` (new ~50 LOC test) OR existing test file
- `scripts/check-routes-catalog-complete.sh` (new ~60 LOC)
- `.github/workflows/gateway-routes-complete.yml` (~25 LOC if CI route taken) OR `scripts/conductor-pre-merge-gate.sh` integration

## Out of scope

- `/api/v1/openapi.json` full OpenAPI 3.x document generation (follow-up brief if catalog-add isn't sufficient)
- `/api/v1/workers` 404 root cause (binary drift) — covered by `fix-gateway-workers-collection-alias-team` + `fix-gateway-binary-freshness-auto-restart-or-warn`
- `/api/v1/connectors` and `/api/v1/tenants` implementation (separate v1.x scope per audit)

## Effort

S–M. ~90 LOC + ~50 LOC test + ~60 LOC script. ~1-2 hours. P1 because the discovery gap silently misleads every fresh agent and every future audit.
