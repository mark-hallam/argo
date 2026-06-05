# sprint-ws5-2-vercel-spa-deploy-verify

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: nothing hard. Cross-links the AWS gateway deploy (WS5-1 #2683) — links resolve once it merges.
**Authored against**: origin/main `d699a56c8` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws5-2-vercel-spa-deploy-verify`.

## Why

`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5-2): *"confirm Vercel deploy
(web/vercel.json → api.skycot.com) + DNS + NEXT_PUBLIC_* prod env."* This is a
**verification** task — a Vercel runbook already exists (`deploy/vercel/README.md`) and
`web/vercel.json` is configured. Verifying it surfaced **two real bugs** in the runbook that
would break a real deploy:

1. **Wrong auth-bypass env var name.** `deploy/vercel/README.md` lists
   `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS`, but the SPA reads
   `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (with `ARGO_`) —
   `web/lib/api/approvals.ts:89`, `web/lib/api/api-keys.ts:92`, +30 more modules.
   `web/vercel.json` already uses the correct name, so it's belt-and-suspenders, but an
   operator following the README table sets a var nothing reads.
2. **Missing Stripe price-id env vars.** The README env table lists
   `NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY` (which is **unused** — `rg` finds zero readers; the
   redirect Checkout goes through the gateway), but **omits**
   `NEXT_PUBLIC_STRIPE_PRICE_PRO` + `NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE`, which the SPA
   actually reads to start Checkout (`web/app/(app)/billing/subscribe/page.tsx:20-21`,
   `web/app/pricing/page.tsx:218`). Without them the Pro/Enterprise CTAs fail.

Plus: the deploy story is scattered (gateway = `infra/terraform/` + AWS runbook; SPA =
`deploy/vercel/`; DNS = `deploy/dns/`; WAF = `deploy/waf/`; secrets = `deploy/secrets/`).
No single operator-facing map ties them together.

## What

1. **Fix `deploy/vercel/README.md`:**
   - Correct the auth-bypass var to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`.
   - Replace the unused `NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY` row with
     `NEXT_PUBLIC_STRIPE_PRICE_PRO` + `NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE` (cross-ref WS4-2:
     create the Stripe products matching `v1-tiers.toml` slugs, put their `price_…` ids here).
   - Add a "how the SPA fits the deploy" cross-link to the gateway side.
2. **New `docs/skycot/deploy/README.md`** — a deploy-overview index tying together gateway
   (AWS), SPA (Vercel), DNS (Cloudflare), WAF, secrets, abuse/cost controls — one operator
   entry point. Flags which pieces are pending-merge.

## May-touch
- `deploy/vercel/README.md` (edit — fix the 2 bugs + cross-link).
- `docs/skycot/deploy/README.md` (NEW — deploy-overview index).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open).

Do NOT change `web/vercel.json` (already correct) or any Rust crate.

## Acceptance criteria
- [ ] `deploy/vercel/README.md` env table names `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (verified against the SPA readers).
- [ ] The env table includes `NEXT_PUBLIC_STRIPE_PRICE_PRO` + `NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE`; the unused publishable-key row is removed or marked unused.
- [ ] `docs/skycot/deploy/README.md` indexes all deploy surfaces with one-line each + the operator order of operations.
- [ ] No Rust file changed; no `web/` code changed (docs only).

## Out of scope
- A live Vercel deploy (operator-gated).
- The AWS gateway IaC (WS5-1), WAF (WS5-4), secrets hardening (WS5-3) — indexed here, shipped elsewhere.
- Wiring the Pro CTA to checkout (WS4-1, already in review) / Stripe product creation (WS4-2, operator).

## Parallel-surface check
- `deploy/vercel/README.md` — exists; this edits it (no parallel).
- `docs/skycot/deploy/README.md` — none today; new index.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5-2). Sprint playbook (WS5).
