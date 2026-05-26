# fix-gateway-cors-localhost-127-and-preview-proxy-origins

**Owner**: codex.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-cors-localhost-127-and-preview-proxy-origins`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F01 [P0] + `scene-1-and-2-claim-1-and-2.md` F6 [P0/P1].

🔴 **P0 DEMO-BLOCKER — Gateway CORS (Cross-Origin Resource Sharing — browser rule controlling cross-origin fetches) allowlist matches `http://localhost:3000` exactly. Any browser that hits the SPA via `127.0.0.1:3000`, the Claude-Preview proxy port (`localhost:53854`), or an OBS browser source on a different port gets `net::ERR_FAILED` on every gateway call. Camera lands on "Worker not found" / perpetual spinners.**

## Root cause

The gateway CORS middleware allowlist contains only the literal string `http://localhost:3000`. The OPTIONS preflight from any other origin returns 204 but omits the `Access-Control-Allow-Origin` header, so the browser silently blocks the follow-up GET/POST. The scene-1/2 audit confirmed: from `localhost:3000` the worker-detail fetch returns 200; from `localhost:53854` (Claude Preview proxy) the same fetch fails with `net::ERR_FAILED`, and the SPA renders "Worker not found / The worker doesn't exist or you don't have access." Curl from a terminal succeeds regardless because curl doesn't enforce CORS.

The two equivalent forms `localhost` vs `127.0.0.1` resolve to the same loopback interface but are treated as distinct origins by the browser. Any dev workflow (Chrome devtools network-throttling profile, OBS browser-source for recording, port-forwarding, alternate Next.js dev port) trips this.

## Curl evidence

```bash
$ curl -s -i -X OPTIONS \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: GET" \
    http://127.0.0.1:7400/api/v1/skills | grep -i access-control
access-control-allow-origin: http://localhost:3000  ← present

$ curl -s -i -X OPTIONS \
    -H "Origin: http://127.0.0.1:3000" \
    -H "Access-Control-Request-Method: GET" \
    http://127.0.0.1:7400/api/v1/skills | grep -i access-control
(no header — browser blocks the request)

$ curl -s -i -X OPTIONS \
    -H "Origin: http://localhost:53854" \
    -H "Access-Control-Request-Method: GET" \
    http://127.0.0.1:7400/api/v1/skills | grep -i access-control
(no header — preview proxy blocked)
```

## Findings this brief closes

- whole-site-mvp-claims-coverage F01 [P0] — gateway CORS allowlist single-origin; every authenticated browser call fails on non-`localhost:3000` origins
- scene-1-and-2-claim-1-and-2 F6 [P0/P1] — `/api/v1/workers/<id>` fails `net::ERR_FAILED` from cross-origin SPA, "Worker not found" lands

## Fix recipe

1. **Locate the CORS middleware** in `crates/argo-gateway/src/` or `crates/argo-cli/src/handlers/gateway/cors*.rs` (grep `Access-Control-Allow-Origin` + `cors`).
2. **Widen the allowlist for dev mode** to include both name and IP forms of localhost across the common dev ports (3000, 3001, 4321, 5173). In dev/auth-bypass mode also allow any `localhost:*` / `127.0.0.1:*` via pattern match. Prod stays strict.
3. **Verify the preflight returns `Access-Control-Allow-Origin`** for each whitelisted origin (echo the requesting origin header, don't hardcode).
4. **Add an integration test** at `crates/argo-cli/tests/gateway_cors_dev_allowlist.rs` that probes OPTIONS with each of `http://localhost:3000`, `http://127.0.0.1:3000`, `http://localhost:53854` and asserts the ACAO header is present + matches the request origin.
5. **Update the demo runbook** `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist with "verify you are recording from `http://localhost:3000` — if you are using OBS browser-source, see the dev CORS note."

## Acceptance criteria

- [ ] OPTIONS preflight returns `Access-Control-Allow-Origin: <requesting-origin>` for all of: `localhost:3000`, `127.0.0.1:3000`, `localhost:53854`, `localhost:3001`.
- [ ] Production builds (`ARGO_ENV=prod` or equivalent) reject non-allowlisted origins as today — no regression on prod isolation.
- [ ] New integration test passes cold.
- [ ] Demo runbook pre-record checklist updated to mention the CORS allowlist.

## May-touch

- `crates/argo-gateway/src/middleware/cors.rs` (or wherever the allowlist lives)
- `crates/argo-cli/src/handlers/gateway/cors_layer.rs` (verify path)
- `crates/argo-cli/tests/gateway_cors_dev_allowlist.rs` (NEW)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (one-line addendum)

## Out of scope

- Production CORS policy review (separate v1.1 hardening pass)
- Authentication / cookie / credentials forwarding (handled in distinct middleware)
- Web smoke tests that assert the SPA loads under each origin variant (deferred until preview MCP is unwedged)

## Effort

~50-100 LOC + 1 integration test. ~1-2 hours. The single highest-leverage demo unblock — without it, the operator's recording environment is silently fragile.
