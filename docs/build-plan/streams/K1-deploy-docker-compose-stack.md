# K1 — Docker Compose deployment stack (argo + web + postgres)

**Owner**: claude. **Depends on**: existing `argo` binary builds via `cargo build --release`; existing `web/` Next.js app builds via `npm run build`. NO existing brief dependency.
**Authored against**: origin/main `08c4ff04` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh K1-deploy-docker-compose-stack`.

> SOURCE: DEPLOYMENT-PATH DIRECTIVE v3 §3 (Tier 5a). **Tier K (deployment recipe).** First of three packaging recipes; produces a single-machine end-to-end deployment via `docker-compose up`. Operator-portable; greenfield-friendly; no orchestrator required.

## Scope

NEW `deploy/docker/` directory (≤ 600 LOC across 6 files):

- `deploy/docker/Dockerfile.argo` (NEW, ≤ 80 LOC) — multi-stage:
  - Build stage: `FROM rust:1.75-slim` + cargo build --release (workspace binary `argo`)
  - Runtime stage: `FROM gcr.io/distroless/cc-debian12` — copy `argo` binary + minimal runtime files
  - HEALTHCHECK CMD `argo doctor --json | grep -q '\"healthy\":true'` every 30s
  - EXPOSE 8080 (gateway)
  - ENTRYPOINT `["argo", "gateway", "serve", "--bind", "0.0.0.0:8080"]`

- `deploy/docker/Dockerfile.web` (NEW, ≤ 80 LOC) — multi-stage:
  - Build stage: `FROM node:20-slim` + `npm ci && npm run build` (Next.js standalone output)
  - Runtime stage: `FROM node:20-slim` — copy `.next/standalone` + `.next/static` + `public/`
  - HEALTHCHECK CMD `wget -qO- http://localhost:3000/ > /dev/null` every 30s
  - EXPOSE 3000
  - ENTRYPOINT `["node", "server.js"]`

- `deploy/docker/docker-compose.yml` (NEW, ≤ 110 LOC) — dev/local default:
  - Service `postgres`: image `postgres:16-alpine`, volume `pg-data`, healthcheck `pg_isready`
  - Service `argo`: build context `..` + `Dockerfile.argo`, depends_on postgres healthy, volume `argo-data` at `/data`, env from `.env`
  - Service `web`: build context `..` + `Dockerfile.web`, depends_on argo healthy, env `NEXT_PUBLIC_GATEWAY_URL=http://argo:8080`
  - Ports: argo 8080:8080, web 3000:3000 (postgres NOT exposed externally — only on the compose network)
  - Volumes: `argo-data`, `pg-data`

- `deploy/docker/docker-compose.prod.yml` (NEW, ≤ 90 LOC) — production overlay:
  - Resource limits (memory, CPU)
  - Restart policy `unless-stopped`
  - Remove host port-binding for postgres (already absent in dev)
  - Add `traefik` labels for TLS-fronted deployment (commented; operator opts in)
  - Logging driver `json-file` with rotation (max 10MB, 5 files)

- `deploy/docker/.env.example` (NEW, ≤ 60 LOC) — documented template:
  - `ARGO_GATEWAY_BEARER_TOKEN=` (required; generate via `openssl rand -hex 32`)
  - `NEXT_PUBLIC_GATEWAY_URL=http://argo:8080` (web→argo on compose net)
  - `DATABASE_URL=postgres://argo:CHANGEME@postgres:5432/argo`
  - `POSTGRES_PASSWORD=CHANGEME` (must match DATABASE_URL above)
  - `ANTHROPIC_API_KEY=` (optional; for live runs)
  - `OPENAI_API_KEY=` (optional)
  - `ARGO_TENANT_DEFAULT=default`
  - `ARGO_LOG_LEVEL=info`

- `deploy/docker/README.md` (NEW, ≤ 180 LOC) — operator runbook:
  - Quickstart: `cp .env.example .env`, edit, `docker-compose up --build`
  - Production: `docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d`
  - Healthcheck verification: `curl http://localhost:8080/api/v1/health` returns 200
  - Smoke test: `curl http://localhost:3000/` returns SPA HTML
  - Volume locations, backup recipe (`docker-compose exec postgres pg_dump`)
  - Upgrade path: `docker-compose pull && docker-compose up -d`
  - Cleanup: `docker-compose down -v` (the `-v` is destructive — flagged loud)
  - Troubleshooting: common cargo build failures, npm cache hits, postgres init race

This is NOT in scope: Helm charts (separate brief — covered by K2 k8s manifests via kustomize); cloud-specific recipes (ECS / GKE / EKS — operators apply K1 directly; cloud-native is K6+ if ever needed); image-publishing to Docker Hub / GHCR (out of scope — operators build locally; image-publish workflow is a follow-up); multi-arch via buildx (build script can be extended; v1 builds for host arch); database migration on container start (handled by argo binary on first connect — no separate migrator job needed).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `deploy/docker/Dockerfile.argo` (NEW, ≤ 80 LOC)
- `deploy/docker/Dockerfile.web` (NEW, ≤ 80 LOC)
- `deploy/docker/docker-compose.yml` (NEW, ≤ 110 LOC)
- `deploy/docker/docker-compose.prod.yml` (NEW, ≤ 90 LOC)
- `deploy/docker/.env.example` (NEW, ≤ 60 LOC)
- `deploy/docker/README.md` (NEW, ≤ 180 LOC)
- `.dockerignore` (EXTEND if exists; NEW otherwise, ≤ 20 LOC — exclude target/, node_modules/, .git, .env)

## Files you must NOT touch

- `crates/**` (no Rust changes)
- `web/app/**` (no frontend changes)
- `Cargo.toml` (no workspace changes)
- `package.json` (no npm changes)

## Acceptance criteria

- [ ] `docker-compose -f deploy/docker/docker-compose.yml build` succeeds from a clean clone
- [ ] `docker-compose -f deploy/docker/docker-compose.yml up -d` reaches all-healthy in ≤ 5 min on a 4-CPU dev host
- [ ] `curl http://localhost:8080/api/v1/health` returns 200
- [ ] `curl -s http://localhost:3000/ | grep -q '<html'` succeeds (SPA HTML served)
- [ ] `docker-compose down -v` cleans state
- [ ] `.env.example` documents every variable with a sensible default or required-note
- [ ] README quickstart verified end-to-end from a fresh machine (developer sanity check)
- [ ] Images < 150 MB each (distroless target for argo; node:slim for web)
- [ ] No new file > 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-K1-deploy-docker-compose-stack`
- PR title: `deploy: docker-compose stack (argo + web + postgres) (K1)`

## Notes for the implementer

- `gcr.io/distroless/cc-debian12` is the correct image — `cc` includes glibc/libstdc++ which Rust binaries link against. `static` would fail on most workspace deps.
- Multi-stage build cache: order Cargo.toml + Cargo.lock COPY BEFORE source COPY so `cargo fetch` layer caches independently. Same for `package.json` + `package-lock.json` BEFORE web source COPY.
- The `argo` binary's `gateway serve` subcommand is the entrypoint — verify subcommand name at claim time via `argo --help` (could be `gateway` or `serve` or `start`).
- The `HEALTHCHECK` line is important — k8s probes (K2) and Docker's restart policy depend on it. Don't skip.
- `DATABASE_URL` connects to postgres on the compose network by service name (`postgres:5432`). This is the docker-compose convention; do NOT use `localhost`.
- Postgres `POSTGRES_PASSWORD` must match the password in `DATABASE_URL` — document this in `.env.example` with a comment.
- The README's "Production" section should warn that the example bearer token is NOT production-safe; operators must generate a new one via `openssl rand -hex 32`.
- DO NOT bake credentials into images. All secrets are env-injected.
- DO NOT include `.env` in `.dockerignore` exclusion if the build process needs build-time env vars — but for K1, all env is runtime. `.env` is OK to exclude from images.
- The K2 (k8s) brief reuses these images by referencing them via OCI registry; document the local image tag convention (`argo:dev`, `argo-web:dev`) in the README so K2 has a stable target.
- The K5 smoke test workflow will `docker-compose up` this stack in CI and run hire→approve checks; keep the compose surface stable across the K-track briefs.
