# SKYCOT Vercel Deployment (single app, both domains)

This runbook configures the `web/` Next.js app. As of the 2026-06-04
single-app consolidation, `web/` is the **only** front-end project and
serves **both** the public marketing site (`skycot.com` apex + `www`)
and the product (`app.skycot.com`). The old separate `marketing/`
project was deleted. Public marketing routes (`/landing`, `/pricing`,
`/about`, `/contact`, `/customers`, `/changelog`, `/features`, `/blog`,
`/privacy`, `/terms`, `/security`, `/sla`, `/dpa`) are auth-exempt in
`web/middleware.ts`; the product surfaces under the `(app)/` route
group stay cookie-gated. See
`docs/skycot/deploy/consolidated-single-app-domain-attach.md` for the
apex domain-attach + DNS steps.

## Project

1. In Vercel, import `github.com/mark-hallam/Argo-Agent`.
2. Set **Root Directory** to `web`.
3. Confirm framework detection is **Next.js**.
4. Keep the defaults from `web/vercel.json`: `pnpm install --frozen-lockfile`, `pnpm build`, output directory `.next`.
5. Leave preview deployments enabled for pull requests.

## Domains

1. Add `app.skycot.com` in Vercel Project Settings -> Domains.
2. Add `skycot.com` (apex) and `www.skycot.com` to the **same** project
   (the single-app consolidation; `www` redirects to the apex). Full
   step-by-step in `docs/skycot/deploy/consolidated-single-app-domain-attach.md`.
3. Keep the Vercel-provided CNAME / apex A-record targets for the DNS brief.
4. Wait for Vercel to issue the managed TLS certificates.
5. Keep `api.skycot.com` and `try.skycot.com` out of this project; they
   are owned by the gateway (AWS Fargate) and demo deployments.

## Environment

Set these in Vercel for Production and Preview:

| Name | Production value | Notes |
|---|---|---|
| `NEXT_PUBLIC_ENV` | `production` | Forces production gateway error behavior. |
| `NEXT_PUBLIC_GATEWAY_URL` | `https://api.skycot.com` | Gateway base URL used by the SPA. |
| `NEXT_PUBLIC_API_URL` | `https://api.skycot.com` | Compatibility alias for deployment tooling. |
| `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` | `0` | Never enable on hosted deployments. |
| `NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY` | Stripe publishable key | Safe public browser key. |
| `NEXTAUTH_URL` | `https://app.skycot.com` | Callback origin. |
| `NEXTAUTH_SECRET` | generated secret | Generate with `openssl rand -base64 32`. |

## Monorepo Deploy Filtering

`web/vercel.json` includes an ignore command that skips builds when
the committed diff does not touch files under the `web/` project root.
Vercel runs it with the project root set to `web`, so backend-only and
documentation-only changes do not redeploy the SPA.

## Verification

After the first preview URL is available:

1. Open the Vercel deployment summary and confirm the root directory is `web`.
2. Check the build log for `pnpm build` and no fallback to `npm install`.
3. Visit `/`, `/team`, and `/audit` on the preview.
4. Run Lighthouse against the preview URL and confirm Performance, Accessibility, Best Practices, and SEO are all at least 95.
