# Single-app consolidation — apex domain attach + DNS (OPERATOR steps)

**Status:** code merged 2026-06-04 (`sprint-consolidate-marketing-into-web-single-app`).
The remaining steps below are **operator/infra only** — they touch the Vercel
dashboard and DNS, which code must not change.

## What changed in code

The old separate `marketing/` Next.js app (package `skycot-marketing`, served
at the `skycot.com` apex) was **deleted**. `web/` is now the single front-end
app and is built to serve **both** domains:

- **`skycot.com` (apex) + `www.skycot.com`** — public marketing site.
- **`app.skycot.com`** — the authenticated product.

The split is **cookie-based, not host-based** (`web/middleware.ts`):

- An unauthenticated visitor hitting `/` is redirected to `/landing` (the
  public marketing home).
- Public marketing routes are auth-exempt and render for anyone:
  `/landing`, `/pricing`, `/about`, `/contact`, `/customers`, `/changelog`,
  `/features`, `/blog`, `/privacy`, `/terms`, `/security`, `/sla`, `/dpa`,
  plus `/docs`, `/compare`, `/teammates`, `/help`, `/for/*`.
- Everything under the `(app)/` route group stays gated behind the session
  cookie and bounces unauthenticated users to `/auth/signin`.

Because the gating is cookie-based, the same deployment behaves correctly on
both hostnames with no host-aware routing required. A visitor to
`skycot.com` lands on `/landing`; a signed-in operator on `app.skycot.com`
sees the product.

## Operator steps (do these in order)

### 1. Attach the apex + www to the `web/` Vercel project

1. Open the **`web/`** Vercel project (the one already serving
   `app.skycot.com`; Root Directory = `web`).
2. **Settings → Domains → Add**:
   - `skycot.com`
   - `www.skycot.com` (set it to **Redirect to `skycot.com`** in Vercel's
     domain settings, 308 — standard apex canonicalization).
3. Vercel will show the required DNS records (apex `A`/`AAAA` + `www` CNAME).
   These already match `deploy/dns/README.md`; no DNS change is needed if the
   apex already pointed at Vercel for the old marketing project. **Confirm**
   the apex A-record `76.76.21.21` and the `www` CNAME `cname.vercel-dns.com`
   are present at the registrar.
4. Wait for Vercel to issue managed TLS for `skycot.com` + `www.skycot.com`.

### 2. Remove the old `marketing` Vercel project

1. Confirm `skycot.com` now serves from the `web/` project (step 1 TLS issued).
2. In the **old `skycot-marketing` Vercel project**: Settings → Domains →
   remove `skycot.com` + `www.skycot.com` (a domain can only be attached to
   one project; Vercel will block step 1 until they are detached here — do
   this first if step 1 reports the domain is in use).
3. Delete the `skycot-marketing` project once the apex is confirmed live on
   `web/`.

### 3. Verify

```bash
# Public apex serves the marketing site (200, not a signin redirect):
curl -sS -o /dev/null -w "%{http_code} %{url_effective}\n" -L https://skycot.com
curl -sS -o /dev/null -w "%{http_code} %{url_effective}\n" -L https://www.skycot.com   # → redirects to apex
for p in pricing about contact customers changelog features blog privacy terms security sla dpa; do
  printf "%-12s " "$p"; curl -sS -o /dev/null -w "%{http_code}\n" "https://skycot.com/$p"
done
# Product still gated on app. (root → /auth/signin when unauthenticated):
curl -sS -o /dev/null -w "%{http_code} %{url_effective}\n" -L https://app.skycot.com
```

Expect `200` on every public route, and `app.skycot.com/` to land on
`/auth/signin` while unauthenticated.

## Notes / future refinements (not launch blockers)

- **SEO duplicate-content.** Because both domains serve identical routes,
  `skycot.com/pricing` and `app.skycot.com/pricing` are the same page on two
  hosts. Each public page already emits a path-relative `canonical` (e.g.
  `/pricing`), which most crawlers resolve per-host. If duplicate-content
  becomes a concern, add a host-aware rewrite in `web/middleware.ts` that
  redirects the public marketing routes on `app.skycot.com` to `skycot.com`
  (and the product routes on the apex to `app.skycot.com`). Deferred — not
  needed for launch.
- **Blog/Features nav.** `/blog` and `/features` were ported from the old
  marketing app but are **not yet in the top nav** (`MarketingNav` routes
  "Features" to `/landing#features`). They are reachable by URL and linked
  from the blog index footer. Add nav entries in
  `web/components/marketing/MarketingNav.tsx` if you want them surfaced.
