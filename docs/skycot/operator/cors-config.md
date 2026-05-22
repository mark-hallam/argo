# CORS allowlist — adding a browser origin

The gateway echoes a request's `Origin` header back in the
`Access-Control-Allow-Origin` response header only when that origin
appears in the `[gateway.cors].allowed_origins` config list.
Browsers refuse to consume responses without a matching ACAO header,
so this list is the authoritative gate for which web UIs are
allowed to call `/api/v1/*`.

The default list — `["http://localhost:3000", "http://localhost:3001"]`
— covers the SPA dev server and Storybook on a freshly checked-out
repo. Production deployments must override it explicitly.

## Add a new origin

1. Edit your operator config (`~/.argo/argo.toml`, or the
   environment-specific file your deploy uses):

   ```toml
   [gateway.cors]
   allowed_origins = [
     "https://app.skycot.com",
     "https://app-staging.skycot.com",
     "http://localhost:3000",
   ]
   ```

2. Restart the gateway. The allowlist is read once at boot — there
   is no runtime reload.

3. Verify with curl from a shell on the gateway host:

   ```bash
   curl -i -X OPTIONS http://127.0.0.1:7400/api/v1/auth/signup \
     -H "Origin: https://app.skycot.com" \
     -H "Access-Control-Request-Method: POST"
   ```

   The response must include `HTTP/1.1 204 No Content` and
   `Access-Control-Allow-Origin: https://app.skycot.com`. If the
   ACAO header is missing, the origin is not in the allowlist and
   browser callers from that origin will see `Failed to fetch` in
   the dev console.

## What the gateway sends back

For every allowlisted Origin, every `/api/v1/*` response (preflight
or real) carries:

| Header                           | Value                                                       |
|----------------------------------|-------------------------------------------------------------|
| `Access-Control-Allow-Origin`    | the request's `Origin` (never `*`)                          |
| `Access-Control-Allow-Methods`   | `GET, POST, PUT, PATCH, DELETE, OPTIONS`                    |
| `Access-Control-Allow-Headers`   | `Content-Type, Authorization, X-Argo-Principal, X-Idempotency-Key, X-Argo-Tenant-Id` |
| `Access-Control-Allow-Credentials` | `true` (cookies + Authorization carried cross-origin)      |
| `Access-Control-Max-Age`         | `86400` (browser-side preflight cache; browser caps apply)   |
| `Vary`                           | `Origin` (prevents shared HTTP caches collapsing responses)  |

`Access-Control-Allow-Origin: *` is never emitted: combining the
wildcard with `Allow-Credentials: true` is rejected by every modern
browser (it would otherwise enable drive-by CSRF reads).

## Failure modes you'll see

- **`Failed to fetch` in the browser dev console, no XHR row at
  all** — the preflight returned without `Access-Control-Allow-Origin`.
  Check the gateway's `[gateway.cors].allowed_origins` list against
  the exact `Origin` string the browser sent (scheme + host + port
  must match — `https://` vs `http://` is a typical miss).

- **`CORS error: 'credentials include' requires explicit origin`** —
  another service or proxy in front of the gateway is rewriting
  ACAO to `*`. Check load-balancer / nginx / caddy config; do not
  let it override the gateway's CORS headers.

- **Preflight works but the real request still fails** — the request
  is hitting `middleware_origin_check`'s CSRF gate. That gate uses
  a separate `ARGO_TRUSTED_ORIGINS` env var (covers POST/PATCH/DELETE
  CSRF protection). Add the origin there as well:

  ```bash
  export ARGO_TRUSTED_ORIGINS="https://app.skycot.com,http://localhost:3000"
  ```

  (The two allowlists are intentionally distinct surfaces today —
  CORS gates browser reads, `ARGO_TRUSTED_ORIGINS` gates CSRF.
  They typically carry the same list.)

## Scope limits

Only `/api/v1/*` routes carry the CORS allowlist. The wizard
endpoints under `/api/v1/wizard/*` use this same allowlist via the
shared dispatcher path; legacy `/sessions/*` and similar non-`/api/v1`
routes do not — they are same-origin only by design.
