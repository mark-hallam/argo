# Wave-4 Security Headers Audit

**Date:** 2026-05-20
**Auditor:** Wave-4 audit agent #3 (Security Headers)
**Scope:** Verify the 6 security headers added to `web/next.config.ts` in commit `c11a0d57` actually serve in production-like requests, and identify any CSP violations the demo might trigger.
**Method:** read-only `curl` probes + static source analysis. No code modified.

---

## 1. Headers actually serve — verdict: **YES on every probed route**

Tested 18 paths against the live dev server. All routes — including 404s, API routes, and static assets under `/_next/*` — return all 6 configured headers:

| Path             | Status | HSTS | X-Frame-Options | X-Content-Type-Options | Referrer-Policy | Permissions-Policy | CSP |
|------------------|--------|------|-----------------|-----------------------|-----------------|--------------------|-----|
| /                | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /landing         | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /compare         | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /pricing         | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /team            | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /audit           | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /workers         | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /skills          | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /cohorts         | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /customers       | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /settings        | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /signup          | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /signin          | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /verify          | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /onboarding      | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /approvals       | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /policies        | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /metrics         | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /demo            | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /favicon.ico     | 404    | yes  | yes             | yes                   | yes             | yes                | yes |
| /_next/static/*  | 200    | yes  | yes             | yes                   | yes             | yes                | yes |
| /api/auth/verify | 400    | yes  | yes             | yes                   | yes             | yes                | yes |

The `source: "/:path*"` glob in `next.config.ts:67` correctly catches every route, including those not yet implemented in the demo (the 404s above are real routes still to be built — `/signup`, `/signin`, `/verify`, `/onboarding`, `/approvals`, `/policies`, `/metrics`, `/demo`).

**No layout or middleware overrides the headers.** Middleware (`web/middleware.ts`) returns `NextResponse.next()` or `NextResponse.redirect(...)`; neither strips the configured response headers.

### Exact CSP value served

```
default-src 'self';
connect-src 'self' http://127.0.0.1:7400 https://api.skycot.com;
img-src 'self' data: blob: https:;
font-src 'self' https://fonts.gstatic.com data:;
style-src 'self' 'unsafe-inline' https://fonts.googleapis.com;
script-src 'self' 'unsafe-inline' 'unsafe-eval';
frame-ancestors 'none';
base-uri 'self';
form-action 'self'
```

---

## 2. CSP violations — predicted count: **0 in dev / 0 in demo, configuration-dependent for error reporting**

Static analysis of `web/app/` + `web/components/` (365 TS/TSX files):

- `dangerouslySetInnerHTML`: **0 callsites**
- `<script>` literal tags in JSX: **0**
- `<iframe>` literal tags in JSX: **0**
- `eval(` callsites: **0**
- Inline `style="..."` attributes: 30 files, all allowed by `style-src 'unsafe-inline'`
- External script imports (`import x from 'https://...'`): **0**
- External `fetch('https://...')`: **0**
- Third-party browser SDKs in `package.json`: **0** (no `@stripe/stripe-js`, no `@sentry/browser`, no Segment / GTM / Mixpanel / Amplitude / Posthog / Datadog / LaunchDarkly / Intercom / HubSpot SDK)

Rendered HTML inspection on `/landing`:

- 19 inline `<script>` tags (no `src`) — React preload hints + Next.js dev runtime + hydration boot. **All allowed** by `script-src 'self' 'unsafe-inline' 'unsafe-eval'`.
- 0 inline `<style>` tags.
- 38 `style="..."` attributes — **all allowed** by `style-src 'self' 'unsafe-inline'`.
- 1 external stylesheet to `https://fonts.googleapis.com/css2?family=Audiowide&...` — **allowed** by `style-src ... https://fonts.googleapis.com`.
- Font files loaded from `https://fonts.gstatic.com` — **allowed** by `font-src ... https://fonts.gstatic.com`.
- HMR (Hot Module Reload) WebSocket connects to `ws://localhost:3000/_next/webpack-hmr` — `connect-src 'self'` covers same-origin WebSockets per modern browser CSP spec.

Cross-page scan (workers, skills, approvals, cohorts, customers, settings, team, audit, pricing, compare) showed identical patterns: only `fonts.googleapis.com`, `fonts.gstatic.com`, and outbound `skycot.com` link `href`s (links don't trigger CSP fetches). **No third-party origins are loaded that are not in the CSP allow-list.**

### Hidden gotcha: error reporting

`web/lib/error-reporting/init.ts` reads a runtime config from the gateway and, if configured, posts crash envelopes to an arbitrary Sentry DSN or OTLP endpoint:

- Currently in dev: `curl /api/v1/meta/error-reporting` → `{"enabled":false,"mode":"disabled"}`. **No CSP violation.**
- In production, when Sentry/OTLP is enabled, the Sentry DSN origin (e.g. `https://o12345.ingest.sentry.io`) and the OTLP endpoint origin will be silently blocked by CSP because `connect-src` only allows `'self'`, `http://127.0.0.1:7400`, and `https://api.skycot.com`.

This is a **configuration-dependent CSP gap**. Fix options when error reporting is turned on:
- (a) extend `connect-src` to include `https://*.ingest.sentry.io` + the configured OTLP endpoint, or
- (b) proxy crash reports through `/api/error-report` and forward server-side (preferred — keeps CSP tight and hides the DSN).

---

## 3. Third-party origins — verdict: **all loaded origins are in the CSP allow-list**

External origins enumerated from the web source:

| Origin                            | Where it appears                    | Loaded as | In CSP? |
|-----------------------------------|-------------------------------------|-----------|---------|
| `https://fonts.googleapis.com`    | `next/font/google` SSR stylesheet   | stylesheet | yes (style-src) |
| `https://fonts.gstatic.com`       | font files referenced from the above stylesheet | font | yes (font-src) |
| `https://api.skycot.com`          | XHR/fetch destination in prod       | fetch     | yes (connect-src) |
| `http://127.0.0.1:7400`           | XHR/fetch destination in dev        | fetch     | yes (connect-src) |
| `https://skycot.com`              | outbound `href` text links          | navigation | no fetch (default-src navigation OK) |
| `https://github.com`, `https://twitter.com`, `https://status.skycot.com`, `https://discord.gg`, `https://help.acmeco.com`, `https://app.linear.app`, `https://dashboard.stripe.com`, `https://console.anthropic.com`, `https://app.hubspot.com`, `https://acmeco.{atlassian.net,lightning.force.com,slack.com,zendesk.com}`, `https://slack.com`, `https://slack-bot.example.com`, `https://hooks.slack.com` | outbound text `href` links | navigation | no fetch (CSP doesn't apply) |
| `https://example.com`, `https://sentry.example`, `https://otel.example`, `http://avatar`, `http://gw.test` | mock data / test fixtures, not used at runtime | — | n/a |

**Verdict: no third-party origin is loaded at runtime that is missing from the CSP allow-list. In the demo path.**

---

## 4. Cache-Control on marketing pages — verdict: **`no-store, must-revalidate` is dev-server behavior, not a config bug**

Every page returns `Cache-Control: no-store, must-revalidate`, including `/landing` (operator's earlier perf audit flagged this). Investigation:

- `next.config.ts` does **not** set `Cache-Control`.
- `web/middleware.ts` does **not** set `Cache-Control`.
- No `export const revalidate = ...` or `export const dynamic = ...` in any `app/**/page.tsx`.
- No `vercel.json` Cache-Control rule.

This means `Cache-Control: no-store, must-revalidate` is the **Next.js dev-server default** when running under `next dev` with Turbopack. The dev server disables caching so Hot Module Reload (HMR — live-updates source code in the browser without a full page reload) works.

**In production (`next start` or Vercel deploy):**
- Static pages will get `Cache-Control: s-maxage=31536000, stale-while-revalidate` automatically.
- Dynamic pages will be re-evaluated on every request, with their own appropriate cache headers.
- Static assets at `/_next/static/*` will get `Cache-Control: public, max-age=31536000, immutable` automatically.

**No fix needed for v1.** Verify post-deploy by re-running `curl -I https://app.skycot.com/landing` and confirming a sensible `Cache-Control`. If we want to be defensive ahead of the demo, we can add explicit `export const revalidate = 3600` to the marketing pages.

---

## 5. Cookies + SameSite — verdict: **enterprise-grade posture, ONE dev caveat**

Cookie surfaces in the SPA:

| Cookie name | Set by | Attributes | Posture |
|-------------|--------|-----------|---------|
| `__Host-session` | Rust gateway via `crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs:125`, forwarded untouched by `web/app/api/auth/verify/route.ts:64` | `Max-Age=...; Path=/; HttpOnly; Secure; SameSite=Lax` | Excellent. `__Host-` prefix is the strictest cookie name binding (requires `Secure`, `Path=/`, no `Domain` — locks cookie to exact origin). Mitigates CSRF + cross-subdomain leak. |
| `authjs.session-token` / `__Secure-authjs.session-token` | NextAuth v5 (default) | Default NextAuth attributes — `HttpOnly`, `SameSite=Lax`, `Secure` in prod | Standard for NextAuth. NextAuth uses `__Secure-` prefix in HTTPS contexts automatically. |

**Caveats:**

1. The `Secure` attribute means the cookie is silently dropped in dev over plain `http://localhost:3000`. This is intentional and not a problem — operator runs the v1 demo over HTTPS via Cloudflare Tunnel / Vercel.
2. No 3rd-party cookies set. No analytics cookies set. No marketing cookies set. Privacy posture is clean.

**Verdict: enterprise-grade. Ship as is.**

---

## 6. Gateway-side headers — verdict: **4 of 6 served on JSON; HSTS+CSP intentionally gated**

`curl -I http://127.0.0.1:7400/api/v1/audit` returns:

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: geolocation=(), camera=(), microphone=(), payment=()
```

Missing on gateway responses:

- **HSTS** — gated to HTTPS-derived requests (reads `X-Forwarded-Proto`). Localhost is HTTP, so absent. In prod behind a TLS-terminating load balancer, the LB sets `X-Forwarded-Proto: https` and HSTS gets added.
- **CSP** — gated to `text/html` responses only. Gateway returns JSON, so absent. Intentional — CSP on a JSON response has no meaning.

Both intentional, both documented at `crates/argo-cli/src/handlers/gateway/security_headers.rs:1-19`. No fix needed.

**CORS:** the gateway has no `OPTIONS` handler and emits no `Access-Control-*` headers. **The dev-mode SPA→gateway connection is intentionally proxied through Next.js (`next.config.ts:30`) to avoid the CORS preflight altogether** (browser asks the server "can I send you this?" before sending the real request; without a CORS response the browser blocks). Production has SPA + gateway colocated behind a single load balancer (same origin), so this is also a non-issue in prod.

**`Cache-Control` on gateway audit endpoint:** not set. The audit endpoint returns potentially sensitive timeline data; downstream HTTP caches could in theory cache it if the response is served over HTTP without further headers. In prod this is mitigated by the bearer-token Authorization header (which makes the response uncacheable per RFC 7234), but a defensive `Cache-Control: private, no-store` on PII-bearing endpoints is a good-hygiene addition for v1.1.

---

## Top 3 gaps to close before demo

1. **None of the demo blocks ship-readiness.** The 6 security headers serve correctly on every demo path. The CSP allow-list correctly covers every actually-loaded origin. No CSP violations are predicted in the demo path. The cookie posture is enterprise-grade.

2. **(Configuration-dependent) Error reporting CSP gap** — when Sentry/OTLP is enabled in prod, crash envelopes will be blocked by CSP because the Sentry DSN origin isn't in `connect-src`. Status: **dev disabled, not a demo blocker**. Recommended fix before turning Sentry on: proxy crash reports through `/api/error-report` route on the SPA (clean fix, keeps DSN private). Tracker brief idea: `claude-security-csp-error-report-proxy`.

3. **(Cosmetic) `vercel.json` duplicates 3 headers but not all 6.** `vercel.json:28-37` declares `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`. The `next.config.ts` `headers()` block is the authority and Vercel honors both, so the runtime result is correct. But the split is confusing for future audits. Recommend: delete the `headers` block from `vercel.json` (let `next.config.ts` be the single source of truth). Not a demo blocker.

## Bonus observation (not in the brief)

Gateway audit endpoint `/api/v1/audit` returns JSON without `Cache-Control: private, no-store`. The response is uncacheable in practice because it requires `Authorization: Bearer ...` (RFC 7234), but it's good hygiene to set this explicitly on PII-bearing endpoints. Tracker brief idea: `claude-gateway-pii-endpoints-no-store`.

---

## Files referenced

- `~/code/argo/web/next.config.ts` — headers() block at line 52
- `~/code/argo/web/middleware.ts` — auth-gate middleware (no header overrides)
- `~/code/argo/web/auth.config.ts` — NextAuth config
- `~/code/argo/web/app/api/auth/verify/route.ts` — forwards `__Host-session` cookie from gateway
- `~/code/argo/web/lib/error-reporting/init.ts` — Sentry/OTLP crash reporter (CSP gap when enabled)
- `~/code/argo/web/vercel.json` — duplicates 3 of 6 headers
- `~/code/argo/crates/argo-cli/src/handlers/gateway/security_headers.rs` — gateway-side headers (4 of 6 always; HSTS+CSP conditional)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs:125` — mints `__Host-session` cookie with full attributes
