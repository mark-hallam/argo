# SKYCOT SPA Vercel Deployment

This runbook configures the `web/` Next.js app for `app.skycot.com`.

## Project

1. In Vercel, import `github.com/mark-hallam/Argo-Agent`.
2. Set **Root Directory** to `web`.
3. Confirm framework detection is **Next.js**.
4. Keep the defaults from `web/vercel.json`: `pnpm install --frozen-lockfile`, `pnpm build`, output directory `.next`.
5. Leave preview deployments enabled for pull requests.

## Domains

1. Add `app.skycot.com` in Vercel Project Settings -> Domains.
2. Keep the Vercel-provided CNAME target for the DNS brief.
3. Wait for Vercel to issue the managed TLS certificate.
4. Keep `skycot.com`, `api.skycot.com`, and `try.skycot.com` out of this project; they are owned by the marketing, gateway, and demo deployments.

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
