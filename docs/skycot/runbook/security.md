# SKYCOT Operator Runbook — Security

Gateway authentication, the public read-only allowlist, and the
integration-test bypass. For SCIM IdP provisioning, see
[SCIM provisioning](scim-provisioning.md). For the full table of
contents, see [RUNBOOK.md](../RUNBOOK.md).

---

## 11. Security

### 11.1 Gateway bearer-token auth

The gateway routes (session creation, message append, run turn,
skill install, memory writes, the read-side dashboards `/sessions`,
`/audit/recent`, `/messages/recent`, `/memory/*`, …) are
anonymously reachable by default — the only protection is the
`127.0.0.1:7400` localhost bind. For any non-localhost deployment,
set a bearer token:

```sh
export ARGO_GATEWAY_BEARER_TOKEN=$(openssl rand -hex 32)
argo gateway start
```

Once the env var is set (with a non-empty value), every POST
request and every non-allowlisted GET request must carry
`Authorization: Bearer <token>` or the gateway returns
`401 Unauthorized` with body
`{"error":"unauthorized","reason":"missing or invalid bearer token"}`.

**Public read-only allowlist** (always anonymous so liveness
probes, load balancers, and catalog-discovery flows keep working):

- `GET /` — root index
- `GET /health`, `/health/live`, `/health/ready`, `/health/full`
- `GET /version`
- `GET /routes`

Everything else (`/sessions`, `/audit/*` except `stream` is gated,
`/messages/*`, `/memory/*`, `/doctor`, `/db/*`, `/metrics/json`,
`/workspace/*`, `/search*`, every per-resource GET, every POST)
is protected.

**Integration-test escape hatch**: set `ARGO_GATEWAY_AUTH_BYPASS=1`
to bypass the check even when a token is configured. Use this in
local CI fixtures that need to exercise GET + POST routes without
threading the token through every test.

The token can also live in `<workspace>/.argo/credentials.toml`
under the same key — `argo_config::cred::get` reads env first
and falls back to that file, matching the precedent for
`ARGO_WEBINBOX_BEARER_TOKEN`.

---

Next: [Where to next + reference](reference.md).
