# Deployed-real E2E lane — runbook

The Playwright suite under `web/tests/e2e/` runs in two distinct lanes:

- **Local lane** (default). 30+ specs that exercise the SPA against a
  mock-fallback path — the gateway client surfaces `NetworkError`/404
  and the approvals API serves bundled sample data. Fast, hermetic,
  runs on every PR.
- **Deployed-real lane** (this doc). 4 specs under
  `web/tests/e2e/deployed/` that point at a real gateway / SPA /
  marketing trio and exercise the full activation path: signup →
  magic-link → wizard → first hire, plus approval round-trip, audit
  chain verification, and Genesis classify. Slow, requires deploy +
  inbox plumbing, runs after `fly deploy` in `deploy-smoke.yml`.

The two lanes prove different things. Keep both.

## Run locally (against any deployed gateway)

```bash
ARGO_E2E_TARGET=deploy \
ARGO_E2E_GATEWAY_URL=https://api.skycot.com \
ARGO_E2E_SPA_URL=https://app.skycot.com \
ARGO_E2E_MARKETING_URL=https://skycot.com \
ARGO_E2E_MAIL_CATCHER_URL=https://mailcatcher.example.com/your-inbox \
ARGO_E2E_ADMIN_TOKEN=<admin-token> \
pnpm --dir web test:e2e:deploy
```

Without `ARGO_E2E_TARGET=deploy`, the deployed specs `test.skip`
themselves; you can safely co-locate them with the local suite.

## Required env

| Var | Purpose | Example |
| --- | --- | --- |
| `ARGO_E2E_TARGET` | `deploy` flips lanes | `deploy` |
| `ARGO_E2E_SPA_URL` / `E2E_SPA_URL` | SPA origin under test | `https://app.skycot.com` |
| `ARGO_E2E_GATEWAY_URL` / `E2E_GATEWAY_URL` | Gateway origin (admin endpoints) | `https://api.skycot.com` |
| `ARGO_E2E_MARKETING_URL` / `E2E_MARKETING_URL` | Marketing site (CTA assertion) | `https://skycot.com` |
| `ARGO_E2E_MAIL_CATCHER_URL` | Inbox proxy (see below) | `https://mailcatcher.example.com/api` |
| `ARGO_E2E_ADMIN_TOKEN` | Bearer for `/admin/e2e/*` endpoints | `op_token_…` |
| `ARGO_E2E_PROVIDER_KEY` | Optional override for stub provider key | `sk-ant-…` |
| `ARGO_E2E_SLACK_AVAILABLE` | `1` enables Slack-side assertions in `hire-flow-real.spec.ts` | unset |
| `E2E_MAGIC_LINK_TIMEOUT_MS` | Inbox poll deadline (default 60s) | `60000` |

## Throwaway-email + mail-catcher options

Every spec mints a fresh address of the form `e2e-test-<ts>-<rand>@throwaway.local`.
The mail-catcher's job is to expose an HTTP JSON inbox for that address.
Two well-trodden setups:

### Option A — smtp4dev in Docker (simplest)

```bash
docker run -d --name skycot-smtp4dev \
  -p 2525:25 -p 5080:80 \
  rnwood/smtp4dev:v3
```

Point the gateway's outbound SMTP at `localhost:2525`, then expose
smtp4dev's REST API as `${ARGO_E2E_MAIL_CATCHER_URL}` (the lane expects
the contract documented in `mail-catcher-deploy.ts`: `GET
/inbox?to=<address>` returns a JSON array with `to`, `html`, `text`,
or `body` fields). A 30-line proxy layer typically suffices.

### Option B — Postmark inbound (cleanest)

If the org already runs Postmark in test mode, configure an inbound
hook that pushes received messages to a small HTTP cache (Cloud Run /
fly micro / etc.) keyed by recipient. Point the lane at that cache.
This is the recommended setup for shared CI runs because Postmark
handles SPF/DKIM and signs the webhook, so emails actually deliver.

### Operator-resume fallback

If neither setup is wired, the lane will throw with a clear
"operator-resume" hint after `E2E_MAGIC_LINK_TIMEOUT_MS`. Pull the
link from any real mail client and paste it into the test runner's
URL prompt to keep the run alive — useful for one-off deploy probes.

## CI wiring

`.github/workflows/deploy-smoke.yml` adds a `Run deployed-real
Playwright lane` step gated on the `ARGO_E2E_DEPLOYED_URL` repo
variable. Set the repo variables + `ARGO_E2E_ADMIN_TOKEN` secret to
opt in. Empty repo var = step skipped (no behavior change).

A red deployed-lane run fails the workflow but **does NOT block PR
merges**. The operator decides: roll back the offending fly deploy,
or push a fix-forward commit. See `incident-response.md` for the
escalation tree.

## Per-run cleanup contract

Each spec calls `DELETE /admin/e2e/tenants` (body `{ email }`) on
completion. The gateway enforces an `e2e-test-` prefix server-side,
so it's impossible to delete production rows via this surface. If
cleanup fails (admin endpoint down, token wrong, etc.), the spec
emits a Playwright `cleanup-miss` annotation rather than failing the
test — the activation-path signal stays green and the operator gets a
visible bread-crumb for the leak.

## Adding a new deployed spec

1. Drop a new file in `web/tests/e2e/deployed/`.
2. Use `resolveEnv()` from `../fixtures/test-env.ts` for URLs.
3. Wrap your assertions in `try { … } finally { deleteTestTenant(…) }`.
4. Skip when `env.mode !== "deploy-smoke"` so the spec is a no-op in
   the local lane.

Keep the lane small. Deployed runs are expensive (real LLM call cost,
real mail traffic, real DB rows); each new spec should justify itself
against a finding from a recent audit, not a hypothetical bug.
