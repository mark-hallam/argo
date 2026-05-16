# codex-hosted-deployment-flyio — Fly.io deployment for argo gateway (Wave 2 P1)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-hosted-deployment-flyio`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5).
- **Self-serve viral readiness** — without hosted backend, no consumer activation.

## Scope

P1. Stand up `api.skycot.com` on Fly.io. Single-region (iad/syd, operator preference) for v1; multi-region post-launch.

Components:
- `Dockerfile` — multi-stage Rust build → distroless runtime. Single binary `argo`. Health endpoint at `/health`.
- `fly.toml` — Fly app config: instance size, health checks, exposed ports (8080 internal → 443 external), Fly Volume for SQLite persistence at `/data`, env var declarations.
- `deploy/flyio/secrets.example.sh` — operator-runnable script to seed Fly secrets (ANTHROPIC_API_KEY, STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, JWT_SIGNING_KEY).
- GitHub Actions workflow `.github/workflows/deploy-prod.yml` — on push to `main` + tag prefix `v*`, deploy via `flyctl deploy`. Initially disabled with `.disabled` suffix; operator activates after first manual deploy.
- Smoke test post-deploy: hit `/health` + `/version` from CI.

LOC: ~400.

## Files you may touch

- `Dockerfile` (NEW at repo root). ≤80 LOC.
- `.dockerignore` (NEW or extend). ≤30 LOC.
- `fly.toml` (NEW at repo root). ≤80 LOC.
- `deploy/flyio/README.md` (NEW) — operator deploy runbook. ≤100 LOC.
- `deploy/flyio/secrets.example.sh` (NEW) — secret-seeding template. ≤40 LOC.
- `deploy/flyio/post-deploy-smoke.sh` (NEW) — smoke check post-deploy. ≤50 LOC.
- `.github/workflows/deploy-prod.yml.disabled` (NEW — operator renames to active when ready). ≤60 LOC.
- `crates/argo-cli/src/handlers/gateway/health.rs` — narrowly: ensure `/health` returns 200 OK with version. ≤30 modified lines (likely already exists).

## Files you must NOT touch

- Other workflow files — F-01's reactivated set is separate.
- Source code beyond the health endpoint — out of scope.
- Database migrations — sqlite on the Fly Volume; no schema change needed.

## Acceptance criteria

- [ ] `docker build .` succeeds locally; image size < 100MB.
- [ ] `fly deploy --dry-run` validates `fly.toml`.
- [ ] `/health` returns 200 with version JSON.
- [ ] Fly Volume mounted at `/data`; SQLite path is `/data/argo.db`.
- [ ] Secrets script documents all required env vars.
- [ ] Post-deploy smoke script hits health + version + a basic API call.
- [ ] `deploy/flyio/README.md` walks operator through first-time setup.
- [ ] `.github/workflows/deploy-prod.yml.disabled` exists; documents the renaming step in its header.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-hosted-deployment-flyio`
- PR title: `infra: Fly.io deploy config for argo gateway (codex-hosted-deployment-flyio)`

## Notes for the implementer

- Distroless runtime: `gcr.io/distroless/cc-debian12` for small + secure base.
- Build stage: `rust:1.82-bookworm-slim` (or current). Use `cargo chef` for layer caching if straightforward.
- Fly instance: `shared-cpu-1x` with 512MB RAM is fine for v1; scale up post-launch.
- Region: ask operator before first deploy; default to `iad` (US East).
- Fly Volume: 10GB initially. Easy to grow later.
- Secret rotation: document in README that JWT_SIGNING_KEY rotation = sign-out everyone. Accepted tradeoff.
- Health endpoint should NOT auth-check — it's the deploy-smoke target.
- Workflow is `.disabled` because operator wants to manually verify the first deploy before automating.
