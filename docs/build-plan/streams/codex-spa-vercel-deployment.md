# codex-spa-vercel-deployment — Vercel deployment for app.skycot.com SPA (Wave 2 P1)

**Owner**: codex. **Depends on**: `claude-brand-identity-system` MERGED (so brand tokens are stable).
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-spa-vercel-deployment`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Commercial usability** — self-serve hosted SPA.
- **Customer-facing — must be fast + reliable.** Vercel edge + Next.js 15 = state-of-the-art delivery.

## Scope

P1. Configure SPA for Vercel:
- `vercel.json` at `web/` root with project settings: framework, build command, output dir, env vars.
- Custom domain mapping: `app.skycot.com` → Vercel project.
- Preview deployments per PR (Vercel default; just verify enabled).
- Edge config for env vars (NEXT_PUBLIC_API_URL etc.).
- Build optimization: SWC, image optimization on, ISR for static-ish pages.
- Deploy webhook for monorepo-aware deploys (only redeploy when web/ changes).

LOC: ~250.

## Files you may touch

- `web/vercel.json` (NEW). ≤60 LOC.
- `web/next.config.ts` — narrowly: ensure `output: 'standalone'` if needed; image domains; experimental flags. ≤30 modified lines.
- `web/.env.example` — narrowly: document all NEXT_PUBLIC_* env vars + server-side env vars. ≤40 LOC.
- `deploy/vercel/README.md` (NEW) — operator setup runbook (domain mapping, env var seeding, GitHub integration). ≤80 LOC.
- `.github/workflows/deploy-spa.yml.disabled` (NEW) — optional GH Action for non-Vercel-native deploy automation. ≤40 LOC.

## Files you must NOT touch

- SPA components, pages, libraries — out of scope.
- Backend Rust — out of scope (separate `codex-hosted-deployment-flyio` brief).
- DNS records — out of scope (`codex-dns-cloudflare-setup` brief).

## Acceptance criteria

- [ ] `vercel.json` exists with valid config (framework auto-detect or `nextjs`).
- [ ] `next.config.ts` reviewed: image domains include `api.skycot.com`, `try.skycot.com`.
- [ ] `.env.example` documents all required env vars with comments explaining each.
- [ ] `deploy/vercel/README.md` walks operator through domain mapping + env seeding step-by-step.
- [ ] `pnpm build` succeeds locally without errors.
- [ ] Lighthouse score (post-deploy, on a deployed preview) ≥ 95 across all 4 metrics.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-spa-vercel-deployment`
- PR title: `infra: Vercel deploy config for app.skycot.com SPA (codex-spa-vercel-deployment)`

## Notes for the implementer

- Vercel auto-detects Next.js; minimal `vercel.json` is fine.
- Preview deployments: enabled by default per PR — no extra config.
- ISR cache: short TTL for now (5min). Tune later.
- Env vars: `NEXT_PUBLIC_API_URL=https://api.skycot.com`, `NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY=<...>`, others as needed.
- Server-side env vars (database URLs etc.) stay non-public.
- After this brief lands, operator does one-time Vercel UI setup: create project, link GitHub repo, add custom domain, paste env vars.
