# Argo — Docker Compose deployment recipe (K1)

Single-machine end-to-end deployment of the argo platform: the `argo`
binary, the Next.js product surface, and a postgres scaffold — brought
up with one `docker-compose up`.

This is the first of the K-track packaging recipes. K2 reuses the same
images as a kustomize-friendly k8s deployment; K5 wires this stack into
CI for hire→approve smoke tests. Keep the compose surface stable.

## Quickstart

```bash
# 1. From the repo root, seed your env file:
cp deploy/docker/.env.example deploy/docker/.env

# 2. Generate a real bearer token and edit deploy/docker/.env:
openssl rand -hex 32  # paste into ARGO_GATEWAY_BEARER_TOKEN=
# Set POSTGRES_PASSWORD to a non-default value and update DATABASE_URL
# to match.

# 3. Build images and bring the stack up:
docker-compose -f deploy/docker/docker-compose.yml --env-file deploy/docker/.env up --build
```

Stack should reach all-healthy in ≤ 5 min on a 4-CPU dev host. Most of
that time is the cold cargo build (~3-4 min); warm rebuilds are <1 min.

## Healthcheck verification

```bash
# Gateway liveness — should return 200:
curl -i http://localhost:8080/health

# Product surface — should serve SPA HTML:
curl -s http://localhost:3000/ | grep -q '<html' && echo "web OK"
```

The argo container also runs `argo gateway probe live` every 30s as its
own healthcheck, so `docker ps` will show `(healthy)` once the gateway
finishes booting. Web container uses `wget` against `/` for the same.

## Production

For long-running deployments, layer the production overlay on top:

```bash
docker-compose -f deploy/docker/docker-compose.yml \
               -f deploy/docker/docker-compose.prod.yml \
               --env-file deploy/docker/.env up -d
```

The overlay adds: resource limits (argo 2GB/2CPU, web 1GB/1CPU,
postgres 1GB/1CPU), restart policy `unless-stopped`, and log rotation
(json-file driver, 10 MB × 5 files).

For TLS-fronted deployments, uncomment the Traefik labels in
`docker-compose.prod.yml` and bring up a Traefik service alongside the
stack — the labels expose argo on `api.example.com` and web on
`example.com`.

**The example `ARGO_GATEWAY_BEARER_TOKEN` in `.env.example` is NOT
production-safe.** Always generate a fresh token via
`openssl rand -hex 32` before deploying outside a dev machine. Same for
`POSTGRES_PASSWORD`.

## Volumes and backups

Two named volumes hold persistent state:

- `argo-data` — mounted at `/data` in the argo container. Holds the
  SQLite store, credentials cache, and any operator-configured files
  under `ARGO_HOME`. Argo today is SQLite-backed; postgres is included
  as scaffolding for the eventual migration.
- `pg-data` — postgres data directory.

Backup recipe (SQLite — the source of truth today):

```bash
docker-compose exec argo tar czf - /data > argo-data-$(date +%Y%m%d).tgz
```

Backup recipe (postgres — useful once argo connects to it):

```bash
docker-compose exec postgres pg_dump -U argo argo > argo-pg-$(date +%Y%m%d).sql
```

## Upgrade path

```bash
docker-compose -f deploy/docker/docker-compose.yml pull
docker-compose -f deploy/docker/docker-compose.yml up -d --build
```

Argo runs schema migrations on first connect, so no separate migrator
job is needed across upgrades. Volume data is preserved.

## Cleanup

```bash
# Stop the stack, keep volumes:
docker-compose -f deploy/docker/docker-compose.yml down

# **DESTRUCTIVE** — also wipes the argo-data and pg-data volumes:
docker-compose -f deploy/docker/docker-compose.yml down -v
```

## Image tag conventions

- `argo:dev` — built locally from this Dockerfile. K2 references the
  same tag from the k8s manifests.
- `argo-web:dev` — same for the web image.

Override via the `image:` keys in `docker-compose.yml` once you're
publishing to a registry. Multi-arch builds via `docker buildx` are
not wired up in v1 — operators build for the host arch.

## Troubleshooting

- **Cargo build fails with `linker not found`.** Distroless runtime
  doesn't ship a linker; build stage uses `rust:1.75-slim` which
  bundles one. If your local cache is stale, `docker-compose build
  --no-cache argo` forces a rebuild.
- **Postgres init race.** First `up` boots postgres, then argo waits
  on the `service_healthy` condition; if postgres takes >50s to pass
  `pg_isready`, argo will time out. Adjust the `start_period` in the
  postgres healthcheck if you're on a slow disk.
- **Web container can't reach argo.** Inside the compose network the
  argo service is reachable at `http://argo:8080`. `localhost:8080`
  works only from the host. `NEXT_PUBLIC_GATEWAY_URL` is embedded at
  build time, so changing it requires `--build` to take effect.
- **Healthcheck flapping during boot.** `start_period=20s` gives argo
  time to initialize; on resource-constrained hosts you may see the
  container marked unhealthy briefly. It self-corrects once the
  gateway binds.

## Sensible deviations from the K1 brief

The brief was authored against a slightly idealized view of the
codebase. The recipe deviates in a few documented ways:

- Brief says `argo gateway serve --bind 0.0.0.0:8080`; actual surface
  is `argo gateway start --bind 0.0.0.0 --port 8080 --insecure-bind`.
- Brief healthcheck shells `argo doctor --json | grep '"healthy":true'`;
  distroless ships no shell or grep, and doctor JSON uses `"ok":true`.
  We use `argo gateway probe live`, which is already in the image and
  exits 0/1.
- Brief health endpoint is `/api/v1/health`; actual is `/health`.
- Web uses pnpm (`web/pnpm-lock.yaml` is canonical), not npm.
- Next standalone output isn't enabled in `web/next.config.ts`, so we
  run `next start` (via `pnpm start`) rather than `node server.js`.
- Compose `build.context` is `../..` (repo root from `deploy/docker/`),
  not `..` — the brief's `..` would resolve to `deploy/`.
- Argo today persists to SQLite via the `argo-data` volume. Postgres
  is included per the brief but currently unused by the binary.
