# SKYCOT go-live readiness — prerequisites tracker (2026-06-15)

Live tracker for the **one-time full-production AWS apply + cold-stranger test**
(the DEC-034 ship gate). Updated as each prerequisite is confirmed. Deploy path = **ECS Fargate + ALB** — **DECIDED 2026-06-15 (operator, Option 1)**: use the already-built + audited full-production path rather than production-ise the single box (validation found the single-box compose is a *gated-demo* config, not self-serve production — demo-mode web, basic-auth wall, gateway in demo-bypass; the repo's DEPLOYMENT.md designates ECS as the 'real production' shape). Route 53 hosted zone: **Z0000000EXAMPLEZONE**. AWS account **123456789012**,
region **ap-southeast-2 (Sydney)**.

| # | Prerequisite | Status | Evidence / notes |
|---|---|---|---|
| 1 | **SES out of sandbox + DKIM** | ✅ **CONFIRMED 2026-06-15** | See SES record below. A cold stranger's signup email will deliver. |
| 2 | **Stripe** (test keys + Pro price) | ✅ **DONE 2026-06-15** | Pro $49/mo USD price `price_EXAMPLE`; test keys `pk_test_…XXXX` + `sk_test_…XXXX` validated + staged in the local go-live values file. Overage price + webhook deferred (webhook needs the live URL; overage not test-triggered). |
| 3 | **DNS / edge** | ✅ **DECIDED 2026-06-15: Route 53 for the test** | skycot.com is on **AWS Route 53**. For the test we stay on Route 53 — it's *less* setup (`route53_zone_id` auto-validates the SSL cert; no domain move, no Cloudflare token). Config: `route53_zone_id=Z0000000EXAMPLEZONE` (auto-validates ACM cert), `front_with_cloudflare=false` + `allow_open_origin_bringup=true` (bring-up mode), api.skycot.com → ALB alias, app.skycot.com → Vercel. **Before going PUBLIC: move to Cloudflare (FREE plan)** for DDoS + the WAF/rate-limit (#2965, free-plan-compatible) + origin-lockdown (#2928). No AWS-WAF module exists, so Route 53 has NO edge rate-limit — fine for a controlled test, add Cloudflare before public. |
| 4 | **SPA hosting** | ✅ **DECIDED 2026-06-15: AWS App Runner** (all-AWS, credit-funded, no Vercel sub — Vercel Hobby blocks commercial use) | Run the web Docker image on App Runner. ⚠️ `NEXT_PUBLIC_*` are **build-time** (Next inlines them) → conductor builds the web image with prod build-args (`NEXT_PUBLIC_GATEWAY_URL=https://api.skycot.com`, `NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY`, `NEXT_PUBLIC_STRIPE_PRICE_PRO`, **no** DEMO/AUTH_BYPASS flags) → push to ECR → App Runner service (port 3000, HTTPS auto) → custom domain `app.skycot.com` (App Runner DNS records → Route 53). Done in the apply co-pilot (CLI for the test; can Terraform-ify later). |
| 6 | **Security gate** (pre-launch audit) | ✅ **GO 2026-06-15** — 4 blockers found + FIXED (#2966-2969), macOS-gated green. See `docs/skycot/audit/2026-06-15-pre-launch-security-audit.md`. |
| 5 | **AWS credits** applied to `...0000` | ✅ **CONFIRMED 2026-06-15** | AWS Activate credits issued and active. Ample — test costs a few dollars; ~5mo of 24/7 runway. |

## Prerequisite 1 — SES (CONFIRMED 2026-06-15)

Verified two independent ways: the operator's SES console dashboard **and** a
read-only `aws sesv2 get-account --region ap-southeast-2` (no email sent):

- **Production access:** ENABLED (out of the sandbox — can email arbitrary recipients, not just verified ones)
- **Sending enabled:** true · **EnforcementStatus:** HEALTHY
- **Daily sending quota:** 50,000 emails / 24h (sandbox would be 200)
- **Max send rate:** 14 emails / second
- **Region:** Asia Pacific (Sydney) — ap-southeast-2
- **Domain:** `skycot.com` — verified identity, **DKIM setup SUCCESS** (AWS Health event `AWS_SES_DKIM_PENDING_TO_VERIFIED`, 2026-06-12)

→ The signup → magic-link email path is production-ready for skycot.com. This was
the prerequisite with the ~24h approval lead, so it is the most important one to
have cleared before the apply.

## Split-deploy wiring (Fargate gateway + Vercel SPA)
- SPA reaches the gateway via **`NEXT_PUBLIC_GATEWAY_URL=https://api.skycot.com`** (Vercel env).
- app.skycot.com ↔ api.skycot.com are same-site (skycot.com), session cookie is `__Host-session; SameSite=Lax` → flows fine.
- **Gateway CORS must allow `https://app.skycot.com`** (not wildcard — wildcard+credentials is forbidden). Wire into the apply config (`GatewayCorsConfig`). 
- SPA must build WITHOUT the demo/bypass flags (`NEXT_PUBLIC_SKYCOT_DEMO_MODE`, `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` unset) for real signup.

## What's left before the apply
Prerequisites 2–5 above. The apply itself is operator-driven (it spends real money
+ enters real credentials); the conductor prepares config/secrets/image/plan and
co-pilots the run live. Cost: a few dollars for a few-hours test (fully covered by
the credits); ~$105–130/mo only if left running 24/7.
