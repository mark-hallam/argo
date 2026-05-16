# Fly.io Production Deploy

This deploys the `argo` gateway for `api.skycot.com` as a single Fly
Machine with a persistent SQLite volume.

## First-Time Setup

1. Install and authenticate `flyctl`.

   ```bash
   brew install flyctl
   fly auth login
   ```

2. Create the app and volume. The default region is `iad`; use `syd`
   instead if the operator wants the v1 deployment near Australia.

   ```bash
   fly apps create skycot-api
   fly volumes create argo_data --app skycot-api --region iad --size 10
   ```

3. Seed secrets.

   ```bash
   export ANTHROPIC_API_KEY=...
   export STRIPE_SECRET_KEY=...
   export STRIPE_WEBHOOK_SECRET=...
   export JWT_SIGNING_KEY=...
   export ARGO_GATEWAY_BEARER_TOKEN=...
   ./deploy/flyio/secrets.example.sh
   ```

4. Validate config, then deploy.

   ```bash
   fly deploy --dry-run --config fly.toml --app skycot-api
   fly deploy --config fly.toml --app skycot-api
   ```

5. Attach `api.skycot.com` after the first deploy succeeds.

   ```bash
   fly certs add api.skycot.com --app skycot-api
   ```

6. Run the post-deploy smoke.

   ```bash
   BASE_URL=https://api.skycot.com ./deploy/flyio/post-deploy-smoke.sh
   ```

## Runtime Notes

- The Fly volume is mounted at `/data`.
- The gateway runs with `ARGO_HOME=/data`, so the current binary writes
  SQLite state to `/data/sessions.db`.
- `fly.toml` keeps one machine warm for production readiness.
- The container listens on `0.0.0.0:8080`, matching Fly's
  `http_service.internal_port`.
- `GET /health`, `GET /version`, and `GET /routes` remain anonymous
  for deploy probes. Authenticated POST routes still require
  `ARGO_GATEWAY_BEARER_TOKEN`.
- Rotating `JWT_SIGNING_KEY` invalidates existing issued sessions.

## Enable GitHub Deploys

The deploy workflow is committed as
`.github/workflows/deploy-prod.yml.disabled`. After the first manual
deploy is verified:

1. Rename it to `.github/workflows/deploy-prod.yml`.
2. Add `FLY_API_TOKEN` to repository secrets.
3. Optionally set repository variables `FLY_APP=skycot-api` and
   `SKY_API_URL=https://api.skycot.com`.
