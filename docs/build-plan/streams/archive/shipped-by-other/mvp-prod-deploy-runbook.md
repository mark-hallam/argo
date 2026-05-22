# mvp-prod-deploy-runbook — production deployment runbook section

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — docs gate for v1.0.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

Operators ramping up Argo in production need a single canonical guide for reverse-proxy + process-supervision + container deployment. RUNBOOK currently has §1-§11; add §12 with three concrete recipes.

## Approach

1. Read existing `docs/skycot/RUNBOOK.md` (and post-#1086 split files under `docs/skycot/runbook/`) to match tone and section conventions.
2. Add new section file `docs/skycot/runbook/production-deploy.md` (≤ 400 LOC) with three recipes:
   - **§12.1 Nginx reverse proxy**: minimal config block (server, location, headers including X-Forwarded-For, basic TLS via certbot).
   - **§12.2 Caddy reverse proxy**: equivalent Caddyfile (TLS auto, headers).
   - **§12.3 systemd unit**: `[Unit]` / `[Service]` with `EnvironmentFile=/etc/argo.env`, `User=argo`, `Restart=on-failure`, journal logging hint.
   - **§12.4 Docker Compose** (rename §12 to be a 4-recipe section): a single-service `compose.yml` with bind-mounted SQLite, env file, healthcheck, port mapping.
3. Cross-link from RUNBOOK index (`docs/skycot/RUNBOOK.md`) under "Operations" or new "Deployment" subsection.
4. Run the quickref generator (`scripts/gen-runbook-quickref.sh`) and commit any updated `quickref.html` so the auto-generated piece is in sync.

## Files you may touch

- `docs/skycot/runbook/production-deploy.md` (NEW, ≤ 400 LOC)
- `docs/skycot/RUNBOOK.md` — narrowly: add link under index
- `docs/skycot/quickref.html` — auto-generated; commit regenerated form

## Files you must NOT touch

- Any source code
- Any other runbook section file beyond cross-links

## Acceptance criteria

- [ ] New section file ≤ 400 LOC (well under 500 ceiling)
- [ ] All 4 recipes (Nginx, Caddy, systemd, Compose) present and complete enough to copy-paste
- [ ] `bash scripts/gen-runbook-quickref.sh` runs cleanly
- [ ] No source/test changes (docs-only PR)

## Branch + PR

- Branch: `mvp-prod-deploy-runbook`
- PR title: `docs: production-deploy runbook §12 (mvp-prod-deploy-runbook)`
