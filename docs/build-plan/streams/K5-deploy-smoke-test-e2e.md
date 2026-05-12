# K5 — Deployment smoke test (end-to-end CI workflow)

**Owner**: claude. **Depends on**: K1 (docker-compose stack); J1 (approvals JSON API); J4 (genesis classify); J5 (workers hire); E0 (frontend foundation); E1-a (approvals gateway wireup); E2-a (hire flow shell). **BLOCKED until all 7 MERGE.**
**Authored against**: origin/main `08c4ff04` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh K5-deploy-smoke-test-e2e`.

> SOURCE: DEPLOYMENT-PATH DIRECTIVE v3 §3 (Tier 5b). **Tier K (deployment recipe).** The acceptance gate for "deployable end-to-end." Runs K1's docker-compose stack in CI, exercises a real hire → approve flow through the deployed gateway + SPA, asserts the round-trip works. First green run cuts the directive's stop condition.

## Scope

NEW `.github/workflows/deploy-smoke.yml` (NEW, ≤ 130 LOC) — workflow:

- Triggers: `push: branches: [main]` + `schedule: cron: '0 4 * * *'` (nightly 4 AM UTC)
- Job `deploy-smoke`:
  - runs-on: ubuntu-latest (uses Docker built-in)
  - steps:
    1. checkout
    2. set up docker buildx
    3. `bash deploy/smoke/run.sh` (timeout 15 min)
    4. on failure: `docker-compose logs > smoke-logs.txt && upload as artifact`
    5. teardown: `docker-compose down -v` (always, even on failure)
  - Concurrency group: `deploy-smoke` (max 1 in flight)

NEW `deploy/smoke/run.sh` (NEW, ≤ 140 LOC) — orchestration:

- Validates: `command -v docker-compose` exists; ports 8080 + 3000 free
- Builds K1 images: `cd deploy/docker && docker-compose build`
- Starts stack: `docker-compose up -d`
- Polls `curl localhost:8080/api/v1/health` until 200 or 60s timeout
- Polls `curl localhost:3000/` until 200 or 60s timeout
- Captures bearer token (from auth-bypass env in compose's `.env.smoke` — DIFFERENT from `.env.example`, this is CI-only)
- Runs `node deploy/smoke/checks.js` (compiled from checks.ts; or runs ts-node directly)
- Reports pass/fail with structured output
- On failure: dumps `docker-compose logs` to stdout, exits 1

NEW `deploy/smoke/checks.ts` (NEW, ≤ 200 LOC) — assertions:

- `health()`: GET `/api/v1/health` → 200, JSON `{healthy: true}`
- `spa()`: GET `/` → 200, HTML contains `<title>Skycot</title>` or matched substring
- `hire_flow()`:
  1. POST `/api/v1/workers` with `{role: 'sales-follow-up', name: 'TestWorker'}` → 201, returns `{worker_id, ...}`
  2. GET `/api/v1/workers/<id>` → 200, returns worker w/ matching name
  3. (mocked approval) POST `/api/v1/inbox/approve` with `{action_id: '<from-fixture>'}` → 200 OR 404 (404 acceptable in v1; the auth + routing is what we're verifying)
- `metrics()`: GET `/metrics` → 200, contains Prometheus exposition format
- All checks emit JSON-line output; exit code 1 on any failure

NEW `deploy/smoke/.env.smoke` (NEW, ≤ 25 LOC) — CI-only env:

- `ARGO_GATEWAY_BEARER_TOKEN=ci-smoke-test-token-do-not-use-in-prod`
- `ARGO_GATEWAY_AUTH_BYPASS=1` (existing escape hatch from mission-gateway-bearer-auth)
- `ARGO_TENANT_DEFAULT=smoke-test`
- `DATABASE_URL=postgres://argo:smoke@postgres:5432/argo_smoke`
- `POSTGRES_PASSWORD=smoke`

NEW `deploy/smoke/README.md` (NEW, ≤ 100 LOC) — operator docs:

- What it does, what it doesn't (NOT a full end-to-end browser test; that's a Playwright follow-up)
- Local debugging: `bash deploy/smoke/run.sh` from a developer machine
- Adding new checks: extend `checks.ts`
- Failure triage: how to read the logs artifact

This is NOT in scope: Playwright browser tests (separate follow-up — K5's checks are API-level only); k8s deployment smoke (k8s is K2's recipe; CI runs docker-compose; a kind-cluster smoke is a follow-up); multi-region / multi-tenant smoke (single-tenant smoke only); LLM live call smoke (workers in this test run mock-mode; live-LLM journey test is mission-real-llm-journey-test, separate brief); audit-chain forensic verification (covered by existing audit-tests); the wider `smoke-skycot.sh` test (that's local-developer; this brief is CI-deployable-stack smoke).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `.github/workflows/deploy-smoke.yml` (NEW, ≤ 130 LOC)
- `deploy/smoke/run.sh` (NEW, ≤ 140 LOC)
- `deploy/smoke/checks.ts` (NEW, ≤ 200 LOC)
- `deploy/smoke/.env.smoke` (NEW, ≤ 25 LOC)
- `deploy/smoke/README.md` (NEW, ≤ 100 LOC)
- `deploy/smoke/package.json` (NEW, ≤ 25 LOC — minimal: `typescript`, `ts-node`, `node-fetch` if needed)
- `deploy/smoke/tsconfig.json` (NEW, ≤ 15 LOC)

## Files you must NOT touch

- `deploy/docker/**` (K1's scope — only consume, don't modify)
- `crates/**` (no Rust changes)
- `web/**` (no frontend changes)
- `.github/workflows/ci.yml` (existing CI; this is a separate workflow)
- `.github/workflows/bench.yml.disabled` (perf bench from mission-bench-perf-regression-detection)

## Acceptance criteria

- [ ] `.github/workflows/deploy-smoke.yml` validates via `actionlint` (no syntax errors)
- [ ] `bash deploy/smoke/run.sh` from a clean clone on a developer machine completes in ≤ 10 min
- [ ] First green run on main CUTS THE DIRECTIVE STOP CONDITION (§5 box 7)
- [ ] Failure produces uploaded `smoke-logs.txt` artifact with docker-compose logs
- [ ] Teardown always runs (success or failure) — no orphan containers
- [ ] All 4 checks (`health`, `spa`, `hire_flow`, `metrics`) pass against a fresh K1 stack
- [ ] Auth-bypass env is documented as CI-ONLY (never used in prod)
- [ ] Concurrency group prevents parallel smoke runs (avoids port conflicts in CI)
- [ ] No new file > 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-K5-deploy-smoke-test-e2e`
- PR title: `deploy: end-to-end deployment smoke test (CI workflow + check script) (K5)`

## Notes for the implementer

- This brief is the directive's stop-condition trigger. Authoring is straightforward; the load-bearing work is making the checks robust against transient startup races (compose stack takes ~30-60s to be ready; polling with timeout is mandatory).
- Choose Node + TypeScript for `checks.ts` because the web stack is already TS — keeps the deploy/smoke surface consistent with the team's existing tooling. Alternative: pure-bash `curl + jq` checks. Less expressive but zero deps. The brief specifies TS but a worker who prefers bash can submit a counter-proposal in PR comments before claiming.
- The `hire_flow()` check is the load-bearing assertion. It exercises the J5 (POST /workers) endpoint and J7 (GET /workers/<id>) endpoint and verifies the full chain through to the gateway. If those endpoints aren't shaped right, smoke fails — which is the point.
- The (mocked approval) step is 200-OR-404 because v1 might not have approval fixtures wired into the smoke; the auth + routing is what matters. v2 brief would inject a real approval and verify the full round-trip.
- `ARGO_GATEWAY_AUTH_BYPASS=1` is the existing escape hatch from `mission-gateway-bearer-auth` (#946). Reusing it for CI avoids manufacturing a bearer-token-acquire flow in the smoke. Document the bypass usage as CI-ONLY.
- The `metrics` endpoint check verifies Prometheus exposition is working. If `mission-prometheus-exporter-default-registry` ships, this check passes naturally. If not yet shipped, the check can be tagged TODO inline.
- Concurrency group `deploy-smoke` prevents two main-push commits from racing into the same compose stack. CI usually serialises pushes anyway, but explicit is better.
- DO NOT add Playwright / browser-level checks in v1. The brief is API-level smoke; full browser tests are a follow-up.
- DO NOT introduce a separate `docker-compose.smoke.yml` overlay. The smoke uses K1's compose file directly with the `.env.smoke` override. Keep the surface area minimal.
- If K1's compose file changes, K5 may need updates. Coordinate via PR comments if smoke fails after a K1 change.
- The `node-fetch` dep can be skipped on Node 18+ which has native fetch. Verify the CI runner's Node version (ubuntu-latest currently runs Node 20+).
- The first green run on main is the moment the directive ends. Conductor watches for the `deploy-smoke` workflow's `conclusion: success` event on main and cuts §5 box 7.
