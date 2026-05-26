# fix-gateway-workers-collection-route-alias-404-add-canonical

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-workers-collection-route-alias-404-add-canonical`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/whole-site-mvp-claims-coverage.md` F8.

🟡 **P1 — `curl http://127.0.0.1:7400/api/v1/workers` returns `{"error":"not found"}` (route doesn't exist) while `curl http://127.0.0.1:7400/api/v1/team` returns 1 real worker ("Maya"). The SPA's `/workers` page calls `/api/v1/workers` (per memory + audit observation), gets 404, and falls back to the fictional 5-roster from `sample-workers-detail.ts`. Real workspace data ("Maya") is invisible. Same root-cause concern as `fix-gateway-workers-collection-alias-team.md` (existing) which appears to have shipped only a sidebar-label change, not the route alias.**

## Root cause

The gateway exposes the worker collection under `/api/v1/team` (per `crates/argo-cli/src/handlers/gateway/routes_team.rs`). The SPA's `fetchWorkers` in `web/lib/api/team.ts` (or equivalent — verify path) calls `/api/v1/workers` which 404s. The fallback in `web/lib/api/team.ts:95-105` (the strict-production guard from prior audits) does NOT fire on a hard 404 — actually verifying behaviour: if the gateway returns 404 (not a 200 with empty payload), the SPA may fall through to the SAMPLE_TEAM fallback regardless.

The existing brief `fix-gateway-workers-collection-alias-team.md` exists in the queue but a search of `crates/argo-cli/src/handlers/gateway/` for a `routes_workers_list.rs` or equivalent canonical route returns no `/api/v1/workers` route (only `/api/v1/team` per the audit's curl proof). The brief appears to have shipped a partial change (sidebar label?), not the gateway route alias itself.

Two reasonable resolutions:
- **(a) Alias `/api/v1/workers` → `/api/v1/team` on the gateway.** Lowest-disruption: add a redirect or duplicate-handler route in the gateway router that responds to `/api/v1/workers` with the same data as `/api/v1/team`.
- **(b) Change the SPA's fetch to call `/api/v1/team`.** Lower API-surface ambiguity going forward. But may affect other SPA consumers that already migrated.

Recommend (a) for v1.0 demo + (b) as a follow-on cleanup once the canonical route name is settled.

## Findings this brief closes

- whole-site-mvp-claims-coverage F8 [P1] — `/api/v1/workers` collection STILL 404 (cumulative across T0519/T0623/T0739)

## Fix recipe

**Path (a) — Add a gateway route alias for `/api/v1/workers`.**

1. In `crates/argo-cli/src/handlers/gateway/routes_team.rs` (or wherever `/api/v1/team` is registered), add a sibling route registration for `/api/v1/workers` that wraps the same handler.
2. Update `crates/argo-cli/src/handlers/gateway/routes.rs` (the router-level registration file — verify path via `rg 'routes_team|api/v1/team' crates/argo-cli/src/handlers/gateway/`) to wire the alias.
3. Cargo test: `curl http://127.0.0.1:7400/api/v1/workers` returns the same payload as `/api/v1/team`.

**Path (b) — Migrate the SPA call.**

4. In `web/lib/api/team.ts` (and any other consumer in `web/lib/api/`), change the fetch URL from `/api/v1/workers` to `/api/v1/team`.
5. Vitest pin: `useTeam()` returns the gateway response.

**Recommended**: ship Path (a) for the demo (smaller blast radius), then file Path (b) as a follow-on `fix-spa-team-api-canonical-route-migration` for post-demo polish.

## Acceptance criteria

- [ ] `curl -s http://127.0.0.1:7400/api/v1/workers` returns a 200 with the same `{total_count, workers: [...]}` payload as `/api/v1/team` (Path a) OR returns 404 with the SPA migrated to call `/api/v1/team` (Path b).
- [ ] In the SPA workers page, the real Maya row is visible (NOT the fictional 5-roster fallback).
- [ ] Cargo test (Path a) protects the alias against regression.
- [ ] In demo mode, the page renders both Maya (real) AND any seeded sample workers, per the existing `auth-bypass` merge pattern.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_team.rs` (alias registration, Path a)
- `crates/argo-cli/src/handlers/gateway/routes.rs` OR equivalent router (route wire-up)
- `web/lib/api/team.ts` (fetch URL, Path b — defer)
- `crates/argo-cli/tests/gateway_workers_alias.rs` (new cargo pin)

## Out of scope

- The demo-seed CLI run that populates the 5-persona roster (covered by `fix-demo-seed-cli-for-real-roster.md`)
- The `/workers` → `/team` redirect behaviour (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker.md` Part 2)
- The sidebar label split between "Workers" and "Team" (covered by existing `fix-gateway-workers-collection-alias-team.md`)

## Effort

S — ~30-50 LOC across gateway + cargo pin. ~1 hour. P1 because Claim 1's "real teammate" promise depends on the real Maya row being visible in the SPA.
