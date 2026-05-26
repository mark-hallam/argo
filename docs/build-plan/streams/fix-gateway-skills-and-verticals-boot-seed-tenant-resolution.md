# fix-gateway-skills-and-verticals-boot-seed-tenant-resolution

**Owner**: codex.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/whole-site-mvp-claims-coverage.md` F1; `scene-4-and-5-claim-4-and-5.md` F7.

🔴 **P0 DEMO-GATE — PR #2407 ("seed demo marketplace skill catalog on gateway boot") merged at 18:16Z. Curl against the live gateway 1h+ later still returns `{"skills":[],"total_count":0,"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e"}`. Same for `GET /api/v1/verticals` → `{"verticals":[]}`. The catalog is empty server-side, not just in the browser. Scene 4 cannot be recorded until skills populate, Scene 3 cannot show real verticals until the catalog populates.**

## Root cause

The boot-time seed ships in the gateway binary post-#2407 (commit `a509bc75e`) but does NOT materialize for the tenant the SPA's `principal_resolve` picks on first hit (`019e5057-1d3f-7fa2-9c1b-c34ef01cc21e`). Three plausible upstream causes (the auditor flagged these as hypotheses; needs source-walk to confirm):

1. **Tenant binding.** The boot seed populates a fixed `tenant_id="default"` or `0190…0000` (the demo tenant constant), but `principal_resolve` mints a different tenant from the dev session (`019e5057-…`). The SPA's `/api/v1/skills` request carries the dev tenant and gets back the empty catalog because the seed lives under a different tenant key.

2. **Feature gate.** The seed is wrapped in `cfg(feature = "demo")` or a `--seed-demo` CLI flag that isn't enabled in the running gateway invocation (per `Makefile`'s `restart-live-gateway` target or `scripts/restart-live-gateway.sh`).

3. **Idempotency sentinel.** The seed runs only on a fresh DB. The current dev gateway boots against a persisted SQLite where the catalog is "already initialised but empty" (a sentinel row exists, the seed skip-fires).

Same root cause applies to `/api/v1/verticals` returning `{"verticals":[]}` — same boot-seed pattern, same empty result.

Evidence:
- `curl -s http://127.0.0.1:7400/api/v1/skills` → 200 `{"skills":[],"total_count":0,"tenant_id":"019e5057-..."}`
- `curl -s http://127.0.0.1:3000/api/v1/skills` (via SPA proxy) → identical
- `curl -s 'http://127.0.0.1:7400/api/v1/skills?q=competitor'` → identical
- Gateway logs at boot: confirm whether seed ran + which tenant

## Findings this brief closes

- whole-site-mvp-claims-coverage F1 [P0] — `/skills` catalog STILL EMPTY through SPA-resolved tenant after #2407 merged
- whole-site-mvp-claims-coverage F10 [P1] — `/api/v1/verticals` returns empty `{"verticals":[]}` while the page renders 6 fixture verticals
- scene-4-and-5-claim-4-and-5 F7 [P1] — Gateway returns empty skills + verticals list on `/api/v1/*` (data-source falls back to mock-offline)

## Fix recipe

1. **Confirm the seed runs at boot** (`crates/argo-cli/src/handlers/gateway/boot.rs` or wherever the seed is invoked).
   - Add a `tracing::info!` line at seed entry + exit logging count of rows inserted + the tenant_id used.
   - Rebuild + restart the gateway. Read logs: did seed fire? Against which tenant?

2. **Resolve tenant binding.** Two options:
   - **(a)** Seed for ALL tenants discovered at boot (iterate `tenants` table). Each tenant gets the canonical 7-skill + 30-vertical catalog inserted if missing.
   - **(b)** Seed only the "demo" tenant constant but have `principal_resolve` map the dev session to that same tenant. This is closer to the original PR #2407 intent but requires the dev session's tenant resolution to match.

   Recommend (a) for resilience — seed runs against every tenant, idempotent on conflict.

3. **Idempotency check.** Seed should use `INSERT … ON CONFLICT DO NOTHING` per row (or `UPSERT` semantics), not a "table-empty? then bulk insert" gate. This handles the "DB already exists but partially seeded" case.

4. **Repro test** in `crates/argo-cli/tests/` or `crates/argo-marketplace/tests/`:
   - Boot the gateway against a fresh SQLite with no rows.
   - Issue `GET /api/v1/skills` with any tenant header.
   - Assert ≥1 row returned.
   - Same for `/api/v1/verticals`.

5. **Operator-facing**: document `make restart-live-gateway` re-fires the seed if rows are absent. Add a freshness check to `scripts/check-gateway-freshness.sh` that asserts `skills.total_count > 0` and `verticals length > 0`.

## Acceptance criteria

- [ ] `curl -s http://127.0.0.1:7400/api/v1/skills` returns ≥3 rows for any tenant id.
- [ ] `curl -s http://127.0.0.1:7400/api/v1/verticals` returns ≥5 rows for any tenant id.
- [ ] Both endpoints return the same data shape they did via the SPA's SEED arrays — so `/skills` and `/onboarding/pick-vertical` render real (not fixture-fallback) data.
- [ ] SPA `data-source` indicator (where present) flips from `mock-offline` to `gateway`.
- [ ] Cargo test asserts ≥1 row in both endpoints on cold boot.
- [ ] Freshness check script updated.

## May-touch

- `crates/argo-cli/src/handlers/gateway/boot.rs` (seed invocation + tenant resolution)
- `crates/argo-marketplace/src/seed.rs` (or wherever the catalog seed lives)
- `crates/argo-store/src/seed/marketplace.rs` (if seed is store-side)
- `crates/argo-cli/tests/gateway_boot_seed_test.rs` (NEW)
- `scripts/check-gateway-freshness.sh` (extend)

## Out of scope

- The SPA-side `/verticals` vs `/api/v1/verticals/catalog` path mismatch (separate brief: `fix-wizard-verticals-rewrite-scope-and-dead-fallback`)
- Adding new skills/verticals to the seed catalog beyond what's there today
- The skill detail Install button (separate brief: `fix-skills-detail-install-button-and-flow`)

## Effort

M — ~150-300 LOC across boot + tests + observability. ~2-4 hours.
