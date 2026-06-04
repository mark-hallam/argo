# sprint-ws2-1-fresh-tenant-seeds-usable-baseline

**Owner**: claude or codex.
**Depends on**: #2676 (MERGED — per-user JWT tenant resolution). Pairs with WS1-2 (#2682, write paths → user tenant).
**Authored against**: origin/main `c9ff99509` on `2026-06-04` (call sites re-verified 2026-06-04: `provision_for_user` @ `crates/argo-identity/src/tenant_provision.rs:86`; provision handler @ `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs:52`; `install_for_tenant_if_absent` @ `crates/argo-skill/src/store_vertical.rs:110`).
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws2-1-fresh-tenant-seeds-usable-baseline`.

## Why

Now that a signed-up user lands in their OWN tenant (keystone #2676) and writes go there
too (WS1-2), that tenant starts completely empty — zero skills, zero verticals, zero
workflows. So a stranger who signs up sees `/skills`, `/workflows`, `/team` blank. WS2-2
gave those pages brand-voiced empty states, but the product feels far more alive if a fresh
tenant is seeded with a small, real, default baseline on provision — the difference between
"empty shell" and "ready to work" in the first 30 seconds.

**NO THEATER:** this must install REAL skills/verticals via the real per-tenant install
path — not fixture/sample data. The seeded items must be genuinely usable.

## What

On tenant provision (`argo_identity::tenant_provision::provision_for_user`, invoked by
`POST /api/v1/onboarding/provision`), install a small default baseline INTO THE NEW TENANT:
- A default **skill set** (a handful of broadly-useful skills) via the existing per-tenant
  install path (`install_for_tenant_if_absent` / the same path `argo-demo-seed` uses — reuse
  it; do NOT invent a parallel seeding path).
- At least **one starter vertical** so `/workflows` + the hire flow have something to show.

Re-verify the exact install API + the provision call site at claim time
(`rg 'provision_for_user|install_for_tenant_if_absent' crates/`). Keep it idempotent (a
re-provision must not duplicate or error). Scope the baseline small + sensible — this is a
starter, not the full catalog.

## May-touch
- `crates/argo-identity/src/tenant_provision.rs` (or wherever provision runs) + the install call.
- possibly a small `crates/argo-demo-seed` or skill-install helper reuse (no parallel seeder).
- `crates/argo-cli/tests/` — provision-then-read test.
- `docs/build-plan/BUILD_STATUS.md` (ledger flip).

## Acceptance criteria
- [ ] After `POST /api/v1/onboarding/provision` for a fresh user, that user's JWT read of `/api/v1/skills` returns the seeded baseline (non-empty) and `/api/v1/verticals` (or workflows) shows ≥1 starter.
- [ ] Idempotent: provisioning the same tenant twice does not duplicate or error.
- [ ] Cross-tenant: tenant B's baseline is independent — seeding A does not appear under B (mirror the existing per-tenant-seed isolation test).
- [ ] `cargo clippy` clean on touched crates; touched + new tests GREEN (report pass lines).
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`.

## Out of scope
- Live provider-key validation (WS2-3) and the empty-state UI (WS2-2, shipped #2678).

## Parallel-surface check
- `rg 'install_for_tenant_if_absent|seed_.*tenant' crates/` — reuse the canonical per-tenant install; no parallel seeder.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS2-1).
