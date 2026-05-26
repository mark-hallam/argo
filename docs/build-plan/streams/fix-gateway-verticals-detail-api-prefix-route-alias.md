# fix-gateway-verticals-detail-api-prefix-route-alias

**Owner**: codex.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-verticals-detail-api-prefix-route-alias`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/onboarding-hire-an-ai-teammate.md` F9 [P1].

🟠 **P1 — Gateway exposes `/verticals` (list, 200) but `/api/v1/verticals/<id>` returns 404. The SPA's `fetchVertical()` hits the api-prefixed singular URL; the list endpoint works because it calls bare `/verticals`. Any hover-preview, "show me what's in this template", or detail-card on /onboarding/pick-vertical that opens the singular path silently fails.**

## Curl evidence

```
$ curl -s -w 'status:%{http_code}\n' http://127.0.0.1:7400/verticals | tail -1
status:200

$ curl -s -w 'status:%{http_code}\n' http://127.0.0.1:7400/api/v1/verticals/marketing-intern
{"error":"not found"} status:404

$ curl -s -w 'status:%{http_code}\n' http://127.0.0.1:7400/verticals/marketing-intern
{"id":"marketing-intern",...} status:200
```

## Root cause

Asymmetric route registration in the gateway dispatch chain. The bare collection `/verticals` and per-id singular `/verticals/<id>` are both mounted; the `/api/v1` prefixed list `/api/v1/verticals` is also mounted (per `fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog`); but `/api/v1/verticals/<id>` singular is NOT mounted.

Compare against `9ea693db1 gateway: alias workers collection route` — same pattern shipped for `/api/v1/workers` aliasing `/api/v1/team`. The aliasing pattern is established; only the verticals singular case is missing.

Callsite in SPA: `web/lib/api/verticals.ts:323` — `apiGet("/api/v1/verticals/<id>")`.

Demo mode masks this because the SPA's local seed fallback fires when the gateway 404s; honest mode lands on the 404.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P1] — Vertical-detail API path inconsistency: list works, detail 404s

## Fix recipe

1. **Locate the verticals dispatch arm** in `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api.rs` (or sibling under `crates/argo-cli/src/handlers/gateway/`). Find the existing `/api/v1/verticals` registration.
2. **Add a `/api/v1/verticals/<id>` arm** that delegates to the same handler the bare `/verticals/<id>` route uses. Mirrors the workers-collection alias shipped in `9ea693db1`.
3. **Integration test** at `crates/argo-cli/tests/gateway_verticals_singular_alias.rs`: assert all four combinations `GET /verticals`, `GET /verticals/<id>`, `GET /api/v1/verticals`, `GET /api/v1/verticals/<id>` return 200 with consistent envelopes for the same vertical id.
4. **Verify SPA caller** at `web/lib/api/verticals.ts:323` no longer needs a fallback path once the alias lands.

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/api/v1/verticals/marketing-intern` returns 200 with the vertical detail envelope.
- [ ] All four route variants return symmetric responses for the same vertical id.
- [ ] New gateway integration test passes cold.
- [ ] SPA's `fetchVertical()` resolves on the api-prefixed path in honest mode.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api.rs` (add singular alias arm)
- `crates/argo-cli/src/handlers/gateway/routes_verticals.rs` (verify handler reachable from both prefixes)
- `crates/argo-cli/tests/gateway_verticals_singular_alias.rs` (NEW)

## Out of scope

- Populating `/api/v1/verticals` list response from registered packs (covered by `fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog`)
- Web rewrite for bare `/verticals` proxying (covered by `fix-web-verticals-route-cors-preflight-symmetric-fix`)
- CORS allowlist widening (covered by `fix-gateway-cors-localhost-127-and-preview-proxy-origins`)

## Effort

~30-60 LOC + 1 integration test. ~1-2 hours. Same shape as the prior workers-collection alias.
