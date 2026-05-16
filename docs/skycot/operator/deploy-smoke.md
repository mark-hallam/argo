# Deploy smoke — running smoke-skycot.sh against a Fly deploy

`scripts/smoke-skycot.sh` covers the local CLI surface (scenarios 1-30).
With the `--target-url <URL>` flag it additionally fires 8 post-deploy
scenarios (31-38) against a deployed SKYCOT instance — Fly app,
staging, or any HTTP-reachable gateway. This page is the operator
runbook for the three cases the swarm watches for.

## Case 1 — Fresh-deploy smoke (post `flyctl deploy`)

After a successful `flyctl deploy` of `apps/skycot-gateway`, run the
deployed scenarios against the Fly app URL to confirm the new image
serves traffic correctly:

```bash
bash scripts/smoke-skycot.sh target/debug/argo \
  --target-url https://api.skycot.com
```

The first 30 scenarios run locally as usual; scenarios 31-38 then
fire against the Fly app. Expected outcomes on a healthy deploy:

- `/healthz`, `/meta`, `/meta/tools` → OK.
- Signup rate-limit (#34) → OK if the gateway enforces burst limits;
  WARN otherwise (informational, doesn't block deploy).
- Happy-path signup (#35) → OK; the throwaway email is consumed once.
- CORS preflight (#36) → OK if `Access-Control-Allow-Origin` is set.
- TLS cert (#37) → OK + prints cert expiry date for ops visibility.
- Latency p95 (#38) → OK (<500ms) or WARN (500–2000ms) on a healthy
  network. FAIL only if p95 > 2000ms.

## Case 2 — Scheduled smoke (cron / GitHub Actions)

The `deploy-smoke` workflow (`.github/workflows/deploy-smoke.yml`)
runs nightly at 04:00 UTC. To also exercise the deployed surface, set
the `FLY_DEPLOYED_URL` repository variable to the Fly app URL — the
workflow's post-deploy smoke step is gated on it being non-empty.

For ad-hoc out-of-band runs, trigger the workflow manually:

```bash
gh workflow run deploy-smoke.yml \
  -f target_url=https://api.skycot.com
```

A green run cuts no DEPLOYMENT-PATH DIRECTIVE box on its own, but a
red run is a P1 page-the-operator signal — the deploy is degraded.

## Case 3 — Failed-deploy diagnosis

When `--target-url` smoke fails, capture the failing scenario number
and the FAIL/WARN reason line. Common patterns:

- `/healthz` non-200: container didn't boot. Check `flyctl logs` for
  the gateway app; usually a panicked `main()` or a missing migration.
- Rate-limit not triggered (WARN): gateway rate-limit middleware
  shipped without enforcement, OR the deploy is behind a CDN that
  absorbs the burst before it hits the origin. Verify via
  `flyctl ssh console` + manual curl loop.
- TLS handshake FAIL: cert expired or SNI mismatch. Run the openssl
  command from scenario 37 by hand against the host to see the full
  cert chain. Re-issue via Fly's TLS panel if expired.
- Latency p95 > 2000ms: usually the database is overloaded or the app
  is cold-started. Check Fly's metrics dashboard and the slow-query
  log on the Postgres app before re-deploying.
- CORS preflight FAIL: security middleware over-tightened — the
  `Access-Control-Allow-Origin` header didn't make it past the
  gateway. Compare the failing deploy's middleware config to the
  last-known-good commit.

Always re-run smoke after the fix lands; the script is idempotent and
the throwaway emails are deterministic only within a single run, so
re-running won't pollute the signup table beyond a handful of rows
per invocation.
