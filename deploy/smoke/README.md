# K5 deployment smoke test

End-to-end smoke test that exercises the K1 docker-compose stack
(`deploy/docker/docker-compose.yml`) and asserts the deployed gateway
+ SPA respond on the documented endpoints.

Runs on every push to `main` and nightly at 04:00 UTC via
`.github/workflows/deploy-smoke.yml`. The first green run on `main`
cuts the DEPLOYMENT-PATH DIRECTIVE v3 §5 box 7 stop condition.

## What it covers

API-level only, no browser layer:

- `GET /health` — liveness + schema check.
- `GET /` — SPA shell loads on :3000.
- `POST /api/v1/workers` (J5 hire) — endpoint reachable + auth path
  honoured. Accepts 2xx OR 4xx; rejects 404 and 5xx.
- `POST /inbox/approve/<id>` — webinbox routing wired. Accepts any
  2xx-4xx response per the brief (the action id is synthetic; the
  point is auth + routing, not a real round-trip).
- `GET /metrics` — Prometheus exposition format reachable.

## What it does NOT cover

- Playwright / browser-level interactions — separate follow-up.
- k8s deployment smoke (`deploy/k8s/`) — separate kind-cluster brief.
- Live-LLM journey — `mission-real-llm-journey-test` covers that.
- Audit-chain forensic verification — covered by `argo-audit` unit
  + integration tests.

## Auth bypass — CI ONLY

`deploy/smoke/.env.smoke` sets `ARGO_GATEWAY_AUTH_BYPASS=1`. This is
the existing escape hatch from `mission-gateway-bearer-auth` (#946)
and **must never be enabled in production**. Reusing it for CI keeps
the smoke test free of bearer-token-acquire ceremony.

## Local debugging

From a clean clone with Docker running:

```bash
bash deploy/smoke/run.sh
```

The script builds images, brings up the compose stack, polls
`localhost:8080/health` and `localhost:3000/` until ready (90s budget
each), runs `checks.ts`, and tears down on exit.

Keep the stack running after a failure for triage:

```bash
SMOKE_KEEP_STACK=1 bash deploy/smoke/run.sh
docker compose -f deploy/docker/docker-compose.yml logs argo
```

## Adding new checks

`checks.ts` is a single file with one function per check, all driven
from the `checks` array in `main()`. To add a check:

1. Write an `async function check<Name>(): Promise<Result>` returning
   `{name, ok, detail}`.
2. Append it to the `checks` array.
3. The harness handles emit + summary + exit code.

Keep new checks API-level (curl-equivalent). Anything UI-driven
belongs in the Playwright follow-up.

## Failure triage

CI uploads `smoke-logs.txt` (full `docker compose logs`) as an
artifact on failure. Open the action run, download the artifact, and
search by service name. Common failure modes:

- `gateway /health` never returns 200 → argo binary panicked at
  startup; grep argo logs for `panicked`.
- `spa /` returns 502/503 → web service couldn't reach argo over the
  compose network; check `NEXT_PUBLIC_GATEWAY_URL` resolution.
- `hire_flow` 404 on `/api/v1/workers` → J5 endpoint regressed; check
  `routes_workers_hire.rs` route registration.

## Concurrency

The workflow's `concurrency: group: deploy-smoke` cap ensures only
one smoke run is in flight at a time. Two pushes landing rapidly will
queue rather than race on ports 8080/3000 in the runner.
